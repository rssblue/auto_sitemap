# auto sitemap

Keep your website's [sitemap](https://en.wikipedia.org/wiki/Sitemaps) up to date by tracking changes in individual pages.

## Install

```
cargo add auto_sitemap
```

## Usage

```rust
// Crawls https://example.com
let mut new_sitemap = auto_sitemap::Sitemap::generate_by_crawling("https://example.com")
    .await
    .unwrap();

// Downloads the old sitemap.
let old_sitemap = auto_sitemap::Sitemap::import("https://example.com/sitemap.xml")
    .await
    .unwrap();

// Combines with URLs from the old site.
// If a hash of a page is different, its `lastmod` value is updated.
new_sitemap.combine_with_old_sitemap(&old_sitemap).unwrap();

// Serializes the sitemap to a string.
let mut buf = std::io::BufWriter::new(Vec::new());
new_sitemap.serialize(&mut buf).unwrap();
println!("{}", String::from_utf8(buf.into_inner().unwrap()).unwrap());
```

```xml
<?xml version="1.0" encoding="utf-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <url>
    <loc>https://example.com/</loc>
    <lastmod>2023-08-13T11:30:46Z</lastmod>
    <xhtml:meta name="auto_sitemap_md5_hash" content="1f0e8893210f6496401d171ff77c7e92" />
  </url>
</urlset>
```

## How does it work?

`auto_sitemap` crawls your website.
Each discovered URL and the hash of the contents of the corresponding page are stored in the sitemap.
When the sitemap is generated the next time, it checks whether the hash for a particular page has changed; if yes, the `lastmod` of the page is updated to present time.
