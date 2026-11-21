use std::path::Path;

use appstream::xmltree::Element;

#[derive(Debug, PartialEq)]
pub struct Pin {
    pub id: String,
    pub url: String,
    pub commit: String,
    pub path: Option<String>,
    pub developer: Element,
    pub project_license: Element,
    pub metadata_license: Option<Element>,
    pub screenshots: Option<Element>,
}

impl Pin {
    pub fn read(path: &Path) -> Result<Self, String> {
        let error = |msg: String| format!("{}: {msg}", path.display());
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let folder = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if stem != folder {
            return Err(error(format!("must sit at bundle/{stem}/{stem}.xml")));
        }
        let bytes = std::fs::read(path).map_err(|err| error(err.to_string()))?;
        Self::parse(&bytes, stem).map_err(error)
    }

    fn parse(bytes: &[u8], expected_id: &str) -> Result<Self, String> {
        let root = Element::parse(bytes).map_err(|err| format!("not valid XML: {err}"))?;
        if root.name != "bundle" {
            return Err("the root element must be <bundle>".into());
        }
        let id = root.attributes.get("id").cloned().unwrap_or_default();
        if id != expected_id {
            return Err(format!("id {id:?} does not match its file name"));
        }

        let source = root.get_child("source").ok_or("needs <source>")?;
        let attr = |key: &str| source.attributes.get(key).cloned().unwrap_or_default();
        if attr("type") != "git" {
            return Err("<source> must be type=\"git\"".into());
        }
        let url = attr("url");
        if !url.starts_with("https://") {
            return Err(format!("source url must be https: {url:?}"));
        }
        // A full sha, so the reviewed content cannot change under the pin.
        let commit = attr("commit");
        if commit.len() != 40
            || !commit
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(format!(
                "commit must be a full 40-char lowercase sha: {commit:?}"
            ));
        }

        let path = source.attributes.get("path").cloned();
        if let Some(path) = &path
            && !is_folder(path)
        {
            return Err(format!(
                "source path must be a folder inside the repo, like dark or themes/dark: {path:?}"
            ));
        }

        let developer = root
            .get_child("developer")
            .filter(|d| d.get_child("name").and_then(text).is_some())
            .cloned()
            .ok_or("needs <developer><name>")?;

        let project_license = root
            .get_child("project_license")
            .filter(|e| text(e).is_some())
            .cloned()
            .ok_or("needs <project_license>, an SPDX expression or Proprietary")?;
        let metadata_license = match root.get_child("metadata_license") {
            Some(e) if text(e).is_none() => return Err("<metadata_license> is empty".into()),
            license => license.cloned(),
        };

        let screenshots = root.get_child("screenshots").cloned();
        if let Some(screenshots) = &screenshots {
            check_screenshots(screenshots)?;
        }

        Ok(Self {
            id,
            url,
            commit,
            path,
            developer,
            project_license,
            metadata_license,
            screenshots,
        })
    }
}

/// no leading slash, no `.` or `..`
fn is_folder(path: &str) -> bool {
    path.split('/').all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
    })
}

/// Each screenshot needs an image, and every image must be an https URL
fn check_screenshots(screenshots: &Element) -> Result<(), String> {
    let mut count = 0;
    for shot in children(screenshots, "screenshot") {
        count += 1;
        let mut images = 0;
        for image in children(shot, "image") {
            images += 1;
            let url = text(image).unwrap_or_default();
            let parsed = appstream::url::Url::parse(&url);
            if !parsed.is_ok_and(|u| u.scheme() == "https") {
                return Err(format!(
                    "a screenshot <image> must be an https URL: {url:?}"
                ));
            }
        }
        if images == 0 {
            return Err("a <screenshot> needs an <image>".into());
        }
    }
    if count == 0 {
        return Err("<screenshots> has no <screenshot>".into());
    }
    Ok(())
}

fn children<'a>(parent: &'a Element, name: &'a str) -> impl Iterator<Item = &'a Element> {
    parent
        .children
        .iter()
        .filter_map(|node| node.as_element())
        .filter(move |e| e.name == name)
}

fn text(element: &Element) -> Option<String> {
    Some(element.get_text()?.trim().to_owned()).filter(|s| !s.is_empty())
}
