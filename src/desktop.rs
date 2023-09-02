use crate::app::App;
use anyhow::Result;

pub struct DesktopGenerator {
    entries: Vec<(String, String)>,
}

impl DesktopGenerator {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn add_entry<K, V>(&mut self, key: K, val: V)
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.entries
            .push((String::from(key.as_ref()), String::from(val.as_ref())));
    }

    /// https://www.freedesktop.org/wiki/Specifications/desktop-entry-spec/
    pub fn generate(mut self, app: &App) -> Result<String> {
        let exec_name = app.executable_name()?;

        self.add_entry("Name", app.product_name());
        self.add_entry("Exec", format!("/usr/bin/{} %U", exec_name));
        self.add_entry("Terminal", "false");
        self.add_entry("Type", "Application");
        self.add_entry("Icon", exec_name);
        if let Some(properties) = app.config().desktop_properties() {
            // order might and will be random. serde_json has `preserve_order` feature,
            // but then EBuilderConfig internally parses it into a HashMap.
            // also the config format might not be json.
            for (key, val) in properties {
                self.add_entry(key, val);
            }
        }
        if let Some(comment) = app.description() {
            self.add_entry("Comment", comment);
        }

        let mut mimes = vec![];
        for protocol in app.config().protocol_associations() {
            for scheme in &protocol.schemes {
                mimes.push(format!("x-scheme-handler/{}", scheme));
            }
        }
        for file_ass in app.config().file_associations() {
            if let Some(mime_type) = &file_ass.mime_type {
                mimes.push(mime_type.clone());
            }
        }
        if !mimes.is_empty() {
            self.add_entry("MimeType", mimes.join(";"));
        }

        let categories = app.config().desktop_categories();
        if !categories.is_empty() {
            self.add_entry("Categories", categories.join(";"));
        }

        let mut contents = String::from("[Desktop Entry]\n");
        for (key, val) in self.entries {
            contents.push_str(&format!("{key}={val}\n"));
        }

        Ok(contents)
    }
}

#[test]
fn test_gen_desktop() -> Result<()> {
    let app: App = App::new_from_package_file("test_assets/package.json")?;

    let generator = DesktopGenerator::new();

    assert_eq!(
        generator.generate(&app)?,
        r#"[Desktop Entry]
Name=Tasje
Exec=/usr/bin/tasje %U
Terminal=false
Type=Application
Icon=tasje
CustomField=custom_value
Comment=Packs Electron apps
MimeType=x-scheme-handler/tasje;x-scheme-handler/ebuilder;x-scheme-handler/electron-builder;application/x-tas
Categories=Tools
"#
    );

    Ok(())
}
