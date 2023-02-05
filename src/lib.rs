//! This is a simple plugin for Perseus that automatically compresses static
//! files after each successful build. Use features to pick between the `brotli`
//! and `gzip` compression algorithms. Brotli is recommended beacuse it's faster,
//! produces smaller files and is supported in everything except Internet Explorer.
//!
//! It can be disabled in development with the `should_run` flag on `CompressionOptions`.
//!
//! # Usage
//!
//! Add the plugin to you Perseus App in your Perseus main function.
//! Note that this will not ensure compressed files are actually served - this
//! has to be set in the router of your server integration.
//!
//! ```
//! # use perseus::PerseusApp;
//! # use perseus::plugins::Plugins;
//! PerseusApp::new()
//!     .plugins(Plugins::new().plugin(
//!         perseus_compress::get_compression_plugin,
//!         perseus_compress::CompressionOptions::default(),
//!     ))
//! # ;
//! ```
//!
//! If you're already using plugins just add the plugin to your `Plugins` as usual.
//!
//! # Configuration
//!
//! Includes and excludes can be defined via file globs. For example,
//! "./dist/static/**/*.css" would match all CSS files in the static output
//! directory while "./dist/static/dont_compress.css" could exclude that specific
//! file.
//!
//! # Quirks
//!
//! Due to some inexplicable behaviour in the `brotli` library, a clean build is
//! required to get the smallest possible WASM binary. Dirty builds will produce
//! a significantly lower compression ratio. If you're about to publish the
//! site, make sure you do a clean build.
//!

use perseus::plugins::{empty_control_actions_registrar, Plugin, PluginEnv};
#[cfg(engine)]
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Options for the auto-compressor.
///
/// # Defaults
///
/// * `include`: `["./dist/static/**/*.css", "./dist/pkg/**/*.wasm", "./dist/pkg/**/*.js"]`
/// * `exclude`: `[]`
/// * `should_run`: `true`
pub struct CompressionOptions<M>
where
    M: AsRef<str> + 'static + Send,
{
    /// Globs for included files
    pub include: Vec<M>,
    /// Globs for excluded files that are matched by the included glob
    pub exclude: Vec<M>,
    /// Should the plugin actually do anything?
    /// Set this via conditional compilation to disable compression in development
    /// but enable it in production.
    ///
    /// This should be unnecessary because brotli is very fast, but if you have
    /// a very large amount of files being compressed it might be useful.
    ///
    /// # Example
    ///
    /// ```
    /// let options = perseus_compress::CompressionOptions {
    ///     should_run: cfg!(not(debug_assertions)),
    ///     ..CompressionOptions::default()
    /// };
    /// ```
    pub should_run: bool,
}

impl Default for CompressionOptions<&'static str> {
    fn default() -> Self {
        Self {
            include: vec![
                "./dist/static/**/*.css",
                "./dist/pkg/**/*.wasm",
                "./dist/pkg/**/*.js",
            ],
            exclude: vec![],
            should_run: true,
        }
    }
}

/// Plugin constructor
pub fn get_compression_plugin<M: AsRef<str> + Send + Sync>() -> Plugin<CompressionOptions<M>> {
    #[allow(unused_mut)]
    Plugin::new(
        "perseus-compress",
        |mut actions| {
            #[cfg(engine)]
            {
                use perseus::plugins::PluginAction;
                actions
                    .build_actions
                    .after_successful_build
                    .register_plugin("perseus-compress", |_, data| {
                        let options = data.downcast_ref::<CompressionOptions<M>>().unwrap();
                        if options.should_run {
                            compress_everything(options)
                        } else {
                            Ok(())
                        }
                    });
                actions
                    .export_actions
                    .after_successful_export
                    .register_plugin("perseus-compress", |_, data| {
                        let options = data.downcast_ref::<CompressionOptions<M>>().unwrap();
                        if options.should_run {
                            compress_everything(options)
                        } else {
                            Ok(())
                        }
                    });
            }
            actions
        },
        empty_control_actions_registrar,
        PluginEnv::Server,
    )
}

#[cfg(engine)]
fn compress_everything<M: AsRef<str> + Send>(
    options: &CompressionOptions<M>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::collections::HashSet;
    use std::fs::File;

    let excludes = options
        .exclude
        .iter()
        .map(|item| glob::glob(item.as_ref()))
        .filter_map(Result::ok)
        .flatten()
        .filter_map(Result::ok)
        .collect::<HashSet<_>>();
    let files = options
        .include
        .iter()
        .map(|item| glob::glob(item.as_ref()))
        .filter_map(Result::ok)
        .flatten()
        .filter_map(Result::ok)
        .filter(|path| !excludes.contains(path));

    for file in files {
        let mut original = File::open(&file)?;
        let out_path = compressed_path(&file);
        let mut out_file = File::create(out_path)?;
        let mut compressed = compressor(&mut out_file);
        std::io::copy(&mut original, &mut compressed)?;
    }
    Ok(())
}

#[cfg(all(engine, feature = "brotli"))]
fn compressed_path(original_path: &Path) -> PathBuf {
    let mut path = original_path.parent().unwrap().to_path_buf();
    path.push(format!(
        "{}.br",
        original_path.file_name().unwrap().to_str().unwrap()
    ));
    path
}

#[cfg(all(engine, feature = "gzip"))]
fn compressed_path(original_path: &Path) -> PathBuf {
    let mut path = original_path.parent().unwrap().to_path_buf();
    path.push(format!(
        "{}.gz",
        original_path.file_name().unwrap().to_str().unwrap()
    ));
    path
}

#[cfg(all(engine, not(any(feature = "gzip", feature = "brotli"))))]
fn compressed_path(_original_path: &Path) -> PathBuf {
    unimplemented!(
        "No compression algorithm set. Please use either the 'gzip' or 'brotli' feature."
    );
}

#[cfg(all(engine, feature = "brotli"))]
fn compressor(file: &mut impl Write) -> impl Write + '_ {
    use brotli::enc::BrotliEncoderParams;
    brotli::CompressorWriter::with_params(file, 4096, &BrotliEncoderParams::default())
}

#[cfg(all(engine, feature = "gzip"))]
fn compressor(file: &mut impl Write) -> impl Write + '_ {
    flate2::write::GzEncoder::new(file, flate2::Compression::default())
}

#[cfg(all(engine, not(any(feature = "gzip", feature = "brotli"))))]
fn compressor(_file: &mut impl Write) -> std::fs::File {
    unimplemented!(
        "No compression algorithm set. Please use either the 'gzip' or 'brotli' feature."
    );
}
