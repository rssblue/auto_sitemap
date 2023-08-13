# auto sitemap

Generate and update your website's sitemap based on when individual pages were updated.

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

// Serialize the sitemap to a string.
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
For each dicovered URL, it stores it in the sitemap together with the hash of the contents of that page.
When the sitemap is generated the next time, it checks whether the hash for a particular page has changed; if yes, the `lastmod` of the page is updated to present.
