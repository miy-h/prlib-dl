mod iip;
mod tile;

use anyhow::anyhow;
use tokio::io::AsyncWriteExt;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

fn extract_html_embeded_str(field: &str, text: &str) -> anyhow::Result<String> {
    let pattern = format!("\"{}\" *: *\"([^\"]+)\"", regex::escape(field));
    let re = regex::Regex::new(&pattern).expect("regex creation should succeed");
    let Some(cap) = re.captures(text) else {
        return Err(anyhow!("field not found: {field}"));
    };
    let result_str = cap.get(1).expect("").as_str().replace("\\/", "/");
    Ok(result_str)
}

async fn extract_iip_settings_from_page(
    client: &reqwest::Client,
    url: &str,
) -> anyhow::Result<iip::Settings> {
    let url_struct = url::Url::parse(url).map_err(|_| anyhow!("Invalid URL: {}", url))?;
    let host = url_struct.host_str();
    if url_struct.scheme() != "https" || host != Some("www.prlib.ru") {
        return Err(anyhow!("Invalid URL: {}", url));
    }

    let invalid_response_error_message = "Invalid response";

    let text = client.get(url).send().await?.text().await?;
    let iip_server_url = extract_html_embeded_str("iipServerURL", &text)
        .map_err(|_| anyhow!(invalid_response_error_message))?;
    let image_dir = extract_html_embeded_str("imageDir", &text)
        .map_err(|_| anyhow!(invalid_response_error_message))?;
    let object_data = extract_html_embeded_str("objectData", &text)
        .map_err(|_| anyhow!(invalid_response_error_message))?;

    Ok(iip::Settings {
        iip_server_url,
        image_dir,
        object_data,
    })
}

fn parse_page_specifier(spec: &str, page_count: u32) -> anyhow::Result<Vec<u32>> {
    let mut result_pages: Vec<u32> = vec![];
    for page_range in spec.split(',') {
        if page_range.contains('-') {
            let num_strings: Vec<_> = page_range.split('-').collect();
            let start: u32 = num_strings
                .first()
                .ok_or(anyhow!("Invalid page specifier"))?
                .parse()?;
            let end_str = num_strings
                .get(1)
                .ok_or(anyhow!("Invalid page specifier"))?;
            if end_str.is_empty() {
                result_pages.extend(start..=page_count);
            } else {
                result_pages.extend(start..=end_str.parse::<u32>()?);
            }
        } else {
            result_pages.push(page_range.parse::<u32>()?);
        }
    }
    Ok(result_pages)
}

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    let target_url = args.get(1).expect("Specify args");
    let dist_dir = args.get(2).expect("Specify args");
    let default_page_spec = String::from("1-");
    let page_spec = args.get(3).unwrap_or(&default_page_spec);

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to build HTTP client");
    println!("Downloading manifest file...");
    let settings = extract_iip_settings_from_page(&client, target_url)
        .await
        .expect("Invalid settings");
    let manifest = iip::parse_manifest(&client, &settings.object_data)
        .await
        .expect("Invalid manifest file");

    let tile_fetcher_semaphore = tokio::sync::Semaphore::new(60);
    let tile_concatenator_semaphore = std::sync::Arc::new(tokio::sync::Semaphore::const_new(1));

    let page_numbers =
        parse_page_specifier(page_spec, manifest.len() as u32).expect("Invalid page specifier");
    for page_num in page_numbers {
        println!("Downloading page {page_num}...");
        let page = manifest.get((page_num - 1) as usize).expect("msg");

        let image = iip::fetch_page(
            &client,
            page,
            &settings,
            &tile_fetcher_semaphore,
            &tile_concatenator_semaphore,
        )
        .await
        .unwrap_or_else(|_| panic!("Download failed: page {page_num}"));

        let mut f = tokio::fs::File::create(format!("{dist_dir}/{page_num}.jpg"))
            .await
            .expect("file open failed");
        f.write_all(&image).await.expect("file write failed");
        f.flush().await.expect("flush failed");
    }
}
