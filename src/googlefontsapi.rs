use std::path::PathBuf;

pub(crate) fn download_to_file(url: &str, path: &PathBuf) {
    dbg!(&path);
    let resp = ureq::get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .call()
        .unwrap();

    let mut file = std::fs::File::create(path).unwrap();
    std::io::copy(&mut resp.into_body().into_reader(), &mut file).unwrap();
}

pub(crate) fn cached_download(url: &str, cache_dir: &PathBuf) -> String {
    let hash = format!("{:x}", md5::compute(url));
    let file = cache_dir.join(&hash);

    if !file.exists() {
        download_to_file(url, &file);
    }
    file.into_os_string().into_string().unwrap()
}