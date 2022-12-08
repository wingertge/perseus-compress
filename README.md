# Perseus Compress

This is a simple plugin for Perseus that automatically compresses static
files after each successful build. Use features to pick between the `brotli`
and `gzip` compression algorithms. Brotli is recommended beacuse it's faster,
produces smaller files and is supported in everything except Internet Explorer.

It can be disabled in development with the `should_run` flag on `CompressionOptions`.

# Usage

Add the plugin to you Perseus App in your Perseus main function.
Note that this will not ensure compressed files are actually served - this
has to be set in the router of your server integration.

```
PerseusApp::new()
    .plugins(Plugins::new().plugin(
        perseus_compress::get_compression_plugin,
        perseus_compress::CompressionOptions::default(),
    ))
```

If you're already using plugins just add the plugin to your `Plugins` as usual.

# Configuration

Includes and excludes can be defined via file globs. For example,
"./dist/static/**/*.css" would match all CSS files in the static output
directory while "./dist/static/dont_compress.css" could exclude that specific
file.

# Quirks

Due to some inexplicable behaviour in the `brotli` library, a clean build is
required to get the smallest possible WASM binary. Dirty builds will produce
a significantly lower compression ratio. If you're about to publish the
site, make sure you do a clean build.
