mod iip;

use std::io::Write;

use anyhow::anyhow;

fn extract_html_embeded_str(field: &str, text: &str) -> anyhow::Result<String> {
    let pattern = format!("\"{}\" *: *\"([^\"]+)\"", field);
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

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    let target_url = args.get(1).expect("Specify args");
    let dist_dir = args.get(2).expect("Specify args");

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to build HTTP client");
    println!("Downloading manifest file...");
    let settings = extract_iip_settings_from_page(&client, target_url)
        .await
        .expect("Invalid settings");
    let manifest = iip::parse_manifest(&client, &settings.object_data)
        .await
        .expect("Invalid manifest file");

    let page = manifest.first().expect("foo");
    println!("Downloading first page...");
    let images = iip::fetch_page(&client, page, &settings)
        .await
        .expect("Download failed");
    for (i, image) in images.iter().enumerate() {
        let mut f =
            std::fs::File::create(format!("{}/{}.jpg", dist_dir, i)).expect("file open failed");
        let b = image.slice(0..image.len());
        f.write_all(&b).expect("file write failed");
        f.flush().expect("file write failed");
    }
}
