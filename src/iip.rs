use anyhow::anyhow;

pub struct Settings {
    pub iip_server_url: String,
    pub image_dir: String,
    pub object_data: String,
}

pub struct Page {
    pub zoom: u64,
    pub width: u64,
    pub height: u64,
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
            .ok_or(anyhow!("Invalid JSON structure"))?;
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
            .floor() as u64;
        let height = max_dimension["h"]
            .as_f64()
            .ok_or(anyhow!("Invalid JSON structure"))?
            .floor() as u64;
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
    zoom: u64,
    index: u64,
) -> anyhow::Result<bytes::Bytes> {
    let fif = format!("{}/{}", settings.image_dir, page.filename);
    let jtl = format!("{},{}", zoom, index);

    let url_str = format!(
        "{}?FIF={}&JTL={}&CVT=JPEG",
        &settings.iip_server_url, fif, jtl
    );

    for _ in 0..10 {
        let response = client.get(&url_str).send().await?;
        if response.status().is_success() {
            let bytes = response.bytes().await?;
            return Ok(bytes);
        }
    }

    Err(anyhow!("max retry"))
}
