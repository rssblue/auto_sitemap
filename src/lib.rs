#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

use chrono::{DateTime, Utc};
use spider::website::Website;
use url::Url;

mod xml;
use crate::xml::SitemapSerde;

/// Sitemap of the website.
#[derive(Debug, PartialEq)]
pub struct Sitemap {
    /// Pages of the website.
    pub pages: Vec<Page>,
}

impl Sitemap {
    /// Generates sitemap by crawling the website.
    pub async fn generate_by_crawling(website_url: impl AsRef<str>) -> Result<Self, String> {
        let website_url = Url::parse(website_url.as_ref()).map_err(|e| e.to_string())?;
        if website_url.scheme() != "http" && website_url.scheme() != "https" {
            return Err("URL should start with http:// or https://".to_string());
        }

        let mut pages = vec![];
        let mut website: Website = Website::new(website_url.as_str());

        website.scrape().await;

        let spider_pages = match website.get_pages() {
            Some(spider_pages) => spider_pages,
            None => return Err("failed to get pages".to_string()),
        };

        for page in spider_pages.iter() {
            let url = Url::parse(page.get_url()).map_err(|e| e.to_string())?;
            let contents = page.get_html().trim().replace("\r\n", "\n"); // normalize line endings
            let hash = md5::compute(contents);
            pages.push(Page {
                url,
                lastmod: Some(chrono::Utc::now()),
                md5_hash: Some(format!("{:x}", hash)),
            });
        }

        Ok(Self { pages })
    }

    /// Imports sitemap from URL or local file.
    pub async fn import(url_or_filepath: impl AsRef<str>) -> Result<Self, String> {
        let url_or_filepath = url_or_filepath.as_ref();
        if url_or_filepath.starts_with("http://") || url_or_filepath.starts_with("https://") {
            Self::import_from_url(url_or_filepath).await
        } else {
            Self::import_from_file(url_or_filepath)
        }
    }

    /// Imports sitemap from URL.
    async fn import_from_url(url: &str) -> Result<Self, String> {
        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("failed to get {}: {}", url, e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("failed to get {}: {}", url, e))?;
        let sitemap = Self::deserialize(&bytes[..])?;

        Ok(sitemap)
    }

    /// Imports sitemap from local file.
    fn import_from_file(filepath: &str) -> Result<Self, String> {
        let file = std::fs::File::open(filepath)
            .map_err(|e| format!("failed to open {}: {}", filepath, e))?;

        let sitemap = Self::deserialize(file)?;

        Ok(sitemap)
    }

    /// Deserializes from XML sitemap.
    /// Additional fields are ignored.
    pub fn deserialize<R: std::io::Read>(reader: R) -> Result<Self, String> {
        let sitemap_serde: SitemapSerde = yaserde::de::from_reader(reader)
            .map_err(|e| format!("failed to deserialize: {}", e))?;

        Self::try_from(sitemap_serde)
    }

    /// Serializes to XML sitemap.
    pub fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), String> {
        let sitemap_serde: SitemapSerde = self.into();

        let yaserde_cfg = yaserde::ser::Config {
            perform_indent: true,
            ..Default::default()
        };
        yaserde::ser::serialize_with_writer(&sitemap_serde, writer, &yaserde_cfg)
            .map_err(|e| format!("failed to serialize: {}", e))?;

        Ok(())
    }

    /// Sorts pages by URL.
    pub fn sort_by_url(&mut self) {
        self.pages.sort_by(|a, b| a.url.cmp(&b.url));
    }

    /// Ignores pages that are missing in the new sitemap.
    /// Uses the old `lastmod` if the hash unchanged, otherwise uses the new `lastmod`.
    pub fn combine_with_old_sitemap(
        &mut self,
        old_sitemap: &Sitemap,
    ) -> Result<UpdateInfo, String> {
        let mut info = UpdateInfo {
            new_pages: vec![],
            updated_pages: vec![],
            unchanged_pages: vec![],
            removed_pages: vec![],
        };

        // HashMap of old URLs and the corresponding `Page`.
        let mut old_pages = old_sitemap
            .pages
            .iter()
            .map(|page| (page.url.clone(), page))
            .collect::<std::collections::HashMap<_, _>>();

        for page in self.pages.iter_mut() {
            match old_pages.remove(&page.url) {
                Some(old_page) => {
                    if let (Some(old_hash), Some(old_lastmod)) =
                        (old_page.md5_hash.clone(), old_page.lastmod)
                    {
                        if Some(old_hash) == page.md5_hash {
                            page.lastmod = Some(old_lastmod);
                            info.unchanged_pages.push(page.url.clone());
                            continue;
                        } else {
                            info.updated_pages.push(page.url.clone());
                        }
                    } else {
                        info.updated_pages.push(page.url.clone());
                    }
                }
                None => info.new_pages.push(page.url.clone()),
            }
        }

        info.removed_pages = old_pages.keys().cloned().collect();

        info.sort();

        Ok(info)
    }

    /// Updates domain of the website for which the sitemap is generated.
    /// Possible use: a sitemap is generated for a locally running website (e.g. localhost:8000),
    /// but the website is deployed to a different domain (e.g. example.com).
    pub fn update_domain(&mut self, new_domain: impl AsRef<str>) -> Result<(), String> {
        let new_domain = new_domain.as_ref();
        let new_domain = Url::parse(new_domain).map_err(|e| e.to_string())?;
        if new_domain.scheme() != "http" && new_domain.scheme() != "https" {
            return Err("URL should start with http:// or https://".to_string());
        }
        let new_scheme = new_domain.scheme();
        let new_host = new_domain.host_str().ok_or("failed to get host")?;
        let new_port = new_domain.port();

        for page in self.pages.iter_mut() {
            let mut url = page.url.clone();
            url.set_host(Some(new_host)).map_err(|e| e.to_string())?;
            url.set_scheme(new_scheme)
                .map_err(|_| "failed to set scheme".to_string())?;
            url.set_port(new_port)
                .map_err(|_| "failed to set port".to_string())?;
            page.url = url;
        }

        Ok(())
    }
}

/// Page of the website.
#[derive(Debug, PartialEq)]
pub struct Page {
    /// Page URL.
    pub url: Url,
    /// Last modification date.
    pub lastmod: Option<DateTime<Utc>>,
    /// MD5 hash of the page contents.
    /// Used to detect changes.
    pub md5_hash: Option<String>,
}

/// Information returned when combining with old sitemap.
#[derive(Debug, PartialEq)]
pub struct UpdateInfo {
    /// URLs of new pages.
    pub new_pages: Vec<Url>,
    /// URLs of updated pages.
    pub updated_pages: Vec<Url>,
    /// URLs of unchanged pages.
    pub unchanged_pages: Vec<Url>,
    /// URLs of removed pages.
    pub removed_pages: Vec<Url>,
}

impl UpdateInfo {
    /// Sorts URLs.
    fn sort(&mut self) {
        self.new_pages.sort();
        self.updated_pages.sort();
        self.unchanged_pages.sort();
        self.removed_pages.sort();
    }
}
