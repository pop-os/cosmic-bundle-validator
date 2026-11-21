use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use appstream::xmltree::{Element, EmitterConfig, XMLNode};
use sha2::{Digest, Sha256};

use crate::pin::Pin;

/// What every theme bundle in the catalog provides.
const PROVIDES_ID: &str = "com.system76.CosmicThemeBundle";
use crate::thumbnail::{HEIGHT, WIDTH};

pub fn write(bundles: &Path, out: &Path, base_url: &str) -> Result<(), String> {
    let mut pins: Vec<PathBuf> = fs::read_dir(bundles)
        .map_err(|err| format!("{}: {err}", bundles.display()))?
        .filter_map(Result::ok)
        .map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();
            entry.path().join(format!("{id}.xml"))
        })
        .filter(|pin| pin.is_file())
        .collect();
    pins.sort();

    let mut catalog = Element::new("components");
    catalog.attributes.insert("version".into(), "1.0".into());
    for path in pins {
        let pin = Pin::read(&path)?;
        let zip = out.join(format!("{}.zip", pin.id));
        let Ok(bytes) = fs::read(&zip) else {
            eprintln!("catalog: skipping {}, it did not build", pin.id);
            continue;
        };
        catalog
            .children
            .push(XMLNode::Element(component(&pin, &bytes, base_url)?));
        println!("catalog    {}", pin.id);
    }

    let path = out.join("catalog.xml");
    let mut xml = Vec::new();
    catalog
        .write_with_config(&mut xml, EmitterConfig::new().perform_indent(true))
        .map_err(|err| format!("catalog: {err}"))?;
    fs::write(&path, xml).map_err(|err| format!("{}: {err}", path.display()))
}

fn component(pin: &Pin, zip: &[u8], base_url: &str) -> Result<Element, String> {
    let name = format!("{}.metainfo.xml", pin.id);
    let mut archive =
        zip::ZipArchive::new(Cursor::new(zip)).map_err(|err| format!("{}: {err}", pin.id))?;
    let mut entry = archive
        .by_name(&name)
        .map_err(|err| format!("{}: {name}: {err}", pin.id))?;
    let date = entry
        .last_modified()
        .map(|t| format!("{:04}-{:02}-{:02}", t.year(), t.month(), t.day()))
        .ok_or_else(|| format!("{}: {name} has no date", pin.id))?;
    let mut meta = Vec::new();
    entry
        .by_ref()
        .take(1_000_000)
        .read_to_end(&mut meta)
        .map_err(|err| format!("{}: {name}: {err}", pin.id))?;
    let mut component = Element::parse(meta.as_slice()).map_err(|err| format!("{name}: {err}"))?;

    let replaced = |name: &str| {
        ["developer", "releases", "icon", "project_license"].contains(&name)
            || (name == "metadata_license" && pin.metadata_license.is_some())
            || (name == "screenshots" && pin.screenshots.is_some())
    };
    component
        .children
        .retain(|node| !matches!(node, XMLNode::Element(e) if replaced(&e.name)));
    let mut icon = text("icon", &format!("{}/{}.svg", base_url, pin.id));
    for (key, value) in [
        ("type", "remote".to_owned()),
        ("width", WIDTH.to_string()),
        ("height", HEIGHT.to_string()),
    ] {
        icon.attributes.insert(key.into(), value);
    }
    component.children.push(XMLNode::Element(icon));
    component
        .children
        .push(XMLNode::Element(pin.developer.clone()));
    let licenses = [Some(&pin.project_license), pin.metadata_license.as_ref()];
    for license in licenses.into_iter().flatten() {
        component.children.push(XMLNode::Element(license.clone()));
    }
    if let Some(screenshots) = &pin.screenshots {
        component
            .children
            .push(XMLNode::Element(screenshots.clone()));
    }

    if component.get_child("provides").is_none() {
        component
            .children
            .push(XMLNode::Element(Element::new("provides")));
    }
    let provides = component.get_mut_child("provides").expect("added above");
    let id = text("id", PROVIDES_ID);
    if !provides
        .children
        .iter()
        .any(|node| node.as_element() == Some(&id))
    {
        provides.children.push(XMLNode::Element(id));
    }

    let mut artifact = Element::new("artifact");
    artifact.attributes.insert("type".into(), "binary".into());
    artifact.children.push(XMLNode::Element(text(
        "location",
        &format!("{}/{}.zip", base_url, pin.id),
    )));
    let mut checksum = text("checksum", &format!("{:x}", Sha256::digest(zip)));
    checksum.attributes.insert("type".into(), "sha256".into());
    artifact.children.push(XMLNode::Element(checksum));
    let mut size = text("size", &zip.len().to_string());
    size.attributes.insert("type".into(), "download".into());
    artifact.children.push(XMLNode::Element(size));

    let mut artifacts = Element::new("artifacts");
    artifacts.children.push(XMLNode::Element(artifact));
    let mut release = Element::new("release");
    release
        .attributes
        .insert("version".into(), crate::source::version(&component)?);
    release.attributes.insert("date".into(), date);
    release.children.push(XMLNode::Element(artifacts));
    let mut releases = Element::new("releases");
    releases.children.push(XMLNode::Element(release));
    component.children.push(XMLNode::Element(releases));
    Ok(component)
}

fn text(name: &str, value: &str) -> Element {
    let mut element = Element::new(name);
    element.children.push(XMLNode::Text(value.to_owned()));
    element
}
