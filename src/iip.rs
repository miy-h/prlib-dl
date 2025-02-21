use crate::tile;
use anyhow::anyhow;

pub struct Settings {
    pub iip_server_url: String,
    pub image_dir: String,
    pub object_data: String,
}

pub struct Page {
    pub zoom: u32,
    pub width: u32,
    pub height: u32,
    pub filename: String,
}

pub async fn parse_manifest(client: &reqwest::Client, url: &str) -> anyhow::Result<Vec<Page>> {
    let mut result_pages: Vec<Page> = vec![];
    let json = client.get(url).send().await?.text().await?;
    let v: serde_json::Value = serde_json::from_str(&json)?;
    let pages = v["pgs"]
        .as_array()
        .ok_or(anyhow!("Invalid JSON structure"))?;
    for page in pages {
        let zoom = page["m"]
            .as_u64()
            .ok_or(anyhow!("Invalid JSON structure"))? as u32;
        let filename = page["f"]
            .as_str()
            .ok_or(anyhow!("Invalid JSON structure"))?
            .to_string();
        let dimensions = page["d"]
            .as_array()
            .ok_or(anyhow!("Invalid JSON structure"))?;
        let max_dimension = dimensions.last().ok_or(anyhow!("Invalid JSON structure"))?;
        let width = max_dimension["w"]
            .as_f64()
            .ok_or(anyhow!("Invalid JSON structure"))?
            .floor() as u32;
        let height = max_dimension["h"]
            .as_f64()
            .ok_or(anyhow!("Invalid JSON structure"))?
            .floor() as u32;
        result_pages.push(Page {
            zoom,
            width,
            height,
            filename,
        });
    }
    Ok(result_pages)
}

pub async fn fetch_tile(
    client: &reqwest::Client,
    page: &Page,
    settings: &Settings,
    zoom: u32,
    index: u32,
    semaphore: &tokio::sync::Semaphore,
) -> anyhow::Result<bytes::Bytes> {
    let permit = semaphore.acquire().await.unwrap();
    let fif = format!("{}/{}", settings.image_dir, page.filename);
    let jtl = format!("{zoom},{index}");

    let url_str = format!(
        "{}?FIF={}&JTL={}&CVT=JPEG",
        &settings.iip_server_url, fif, jtl
    );

    for i in 0..10 {
        if let Ok(response) = client.get(&url_str).send().await {
            if response.status().is_success() {
                if let Ok(bytes) = response.bytes().await {
                    drop(permit);
                    return Ok(bytes);
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(std::cmp::min(
            250 * 2u64.pow(i),
            5000,
        )))
        .await;
    }

    drop(permit);
    Err(anyhow!("max retry"))
}

pub async fn fetch_page_tiles(
    client: &reqwest::Client,
    page: &Page,
    settings: &Settings,
    tile_fetcher_semaphore: &tokio::sync::Semaphore,
) -> anyhow::Result<Vec<bytes::Bytes>> {
    let tile_size: u32 = 256;
    let horizontal_count = page.width.div_ceil(tile_size);
    let vertical_count = page.height.div_ceil(tile_size);

    let futures: Vec<_> = (0..(horizontal_count * vertical_count))
        .collect::<Vec<_>>()
        .iter()
        .map(|i| {
            fetch_tile(
                client,
                page,
                settings,
                page.zoom,
                *i,
                tile_fetcher_semaphore,
            )
        })
        .collect();
    let results = futures::future::join_all(futures)
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(results)
}

pub async fn fetch_page(
    client: &reqwest::Client,
    page: &Page,
    settings: &Settings,
    tile_fetcher_semaphore: &tokio::sync::Semaphore,
    tile_concatenator_semaphore: &std::sync::Arc<tokio::sync::Semaphore>,
) -> anyhow::Result<bytes::Bytes> {
    let width = page.width;
    let height = page.height;
    let images = fetch_page_tiles(client, page, settings, tile_fetcher_semaphore).await?;
    let permit = tile_concatenator_semaphore
        .clone()
        .acquire_owned()
        .await
        .unwrap();
    tokio::task::spawn_blocking(move || {
        let image = tile::concat_jpeg_tile(width, height, &images);
        drop(permit);
        image
    })
    .await?
}
