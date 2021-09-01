use super::{Context, Module, RootModuleConfig};

use crate::configs::pony::PonyConfig;
use crate::formatter::StringFormatter;
use crate::formatter::VersionFormatter;

/// Creates a module with the current Pony version
pub fn module<'a>(context: &'a Context) -> Option<Module<'a>> {
    let mut module = context.new_module("pony");
    let config = PonyConfig::try_load(module.config);

    let is_pony_project = context
        .try_begin_scan()?
        .set_files(&config.detect_files)
        .set_extensions(&config.detect_extensions)
        .set_folders(&config.detect_folders)
        .is_match();

    if !is_pony_project {
        return None;
    }

    let parsed = StringFormatter::new(config.format).and_then(|formatter| {
        formatter
            .map_meta(|variable, _| match variable {
                "symbol" => Some(config.symbol),
                _ => None,
            })
            .map_style(|variable| match variable {
                "style" => Some(Ok(config.style)),
                _ => None,
            })
            .map(|variable| match variable {
                "version" => {
                    let output = context.exec_cmd("ponyc", &["--version"])?.stdout;
                    let pony_version = output.trim().split('-').next()?;
                    VersionFormatter::format_module_version(
                        module.get_name(),
                        pony_version,
                        config.version_format,
                    )
                    .map(Ok)
                }
                _ => None,
            })
            .parse(None)
    });

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `pony`:\n{}", error);
            return None;
        }
    });

    Some(module)
}

#[cfg(test)]
mod tests {
    use crate::test::ModuleRenderer;
    use ansi_term::Color;
    use std::fs::File;
    use std::io;

    #[test]
    fn folder_without_pony() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("pony.txt"))?.sync_all()?;
        let actual = ModuleRenderer::new("pony").path(dir.path()).collect();
        let expected = None;
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_pony_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("main.pony"))?.sync_all()?;
        let actual = ModuleRenderer::new("pony").path(dir.path()).collect();
        let expected = Some(format!("via {}", Color::Yellow.bold().paint("↯ v0.6.0 ")));
        assert_eq!(expected, actual);
        dir.close()
    }
}
