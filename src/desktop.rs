use crate::types::{EBFileAssoc, EBProtocol, EBuilderConfig, PackageJson};

/// https://www.freedesktop.org/wiki/Specifications/desktop-entry-spec/
pub fn gen_dotdesktop(ebuilder: &EBuilderConfig, package: &PackageJson) -> (String, String) {
    let eb_linux = ebuilder.linux.clone().unwrap_or_default();
    let exec_name = eb_linux
        .executable_name
        .as_ref()
        .or(ebuilder.executable_name.as_ref())
        .unwrap_or(&package.name);
    let mut lines = vec![
        "[Desktop Entry]".to_string(),
        format!(
            "Name={}",
            ebuilder
                .product_name
                .as_ref()
                .or_else(|| package.product_name.as_ref())
                .unwrap_or(&package.name),
        ),
        format!("Exec=/usr/bin/{} %U", exec_name),
        "Terminal=false".to_string(),
        "Type=Application".to_string(),
        format!("Icon={}", exec_name),
    ];
    if let Some(properties) = eb_linux.desktop {
        for (key, val) in properties {
            lines.push(format!("{}={}", key, val));
        }
    }
    if let Some(comment) = &package.description {
        lines.push(format!("Comment={}", comment));
    }
    let mut mimes = vec![];
    if let Some(protocols) = eb_linux.protocols.or_else(|| ebuilder.protocols.clone()) {
        for protocol in Vec::<EBProtocol>::from(protocols) {
            for scheme in protocol.schemes {
                mimes.push(format!("x-scheme-handler/{}", scheme));
            }
        }
    }
    if let Some(file_assocs) = eb_linux
        .file_associations
        .or_else(|| ebuilder.file_associations.clone())
    {
        for file_ass in Vec::<EBFileAssoc>::from(file_assocs) {
            if let Some(mime_type) = file_ass.mime_type {
                mimes.push(mime_type);
            }
        }
    }
    if mimes.len() > 0 {
        lines.push(format!("MimeType={};", mimes.join(";")));
    }

    if let Some(categories) = eb_linux.category {
        lines.push(format!("Categories={}", categories));
    }
    // end with empty line
    lines.push("".to_string());

    (format!("{}.desktop", package.name), lines.join("\n"))
}

#[test]
fn test_gen_dotdesktop() {
    use crate::types::PackageJson;

    let package: PackageJson =
        serde_json::from_str(include_str!("test_assets/package.json")).unwrap();

    let (dotdesktop_name, dotdesktop_content) =
        gen_dotdesktop(package.build.as_ref().unwrap(), &package);

    assert_eq!(dotdesktop_name, "electron_tasje.desktop");
    assert_eq!(
        dotdesktop_content,
        r#"[Desktop Entry]
Name=Tasje
Exec=/usr/bin/tasje %U
Terminal=false
Type=Application
Icon=tasje
CustomField=custom_value
Comment=Packs Electron apps
MimeType=x-scheme-handler/tasje;x-scheme-handler/ebuilder;x-scheme-handler/electron-builder;application/x-tas;
Categories=Tools
"#
    );
}
