# text2svg

A tool help to convert text to svg file with highlighting support.

## Usage

```
A command line tool help convert text to svg file

Usage: text2svg [OPTIONS] [TEXT]

Arguments:
  [TEXT]  input text string

Options:
      --width <WIDTH>              max width per line (characters)
      --pixel-width <PIXEL_WIDTH>  max width per line (pixels)
  -f, --file <FILE>                input file
  -o, --output <OUTPUT>            output svg file path [default: output.svg]
      --font <FONT>                font family name (e.g., "Arial", "Times New Roman")
      --size <SIZE>                font size in pixels [default: 64]
      --fill <FILL>                svg fill color (e.g., "#ff0000", "none"). Overridden by highlight [default: none]
      --color <COLOR>              font stroke color (e.g., "#000", "currentColor"). Overridden by highlight [default: #000]
      --animate                    Add progressive line-by-line draw animation effect (works best with stroke only)
      --style <STYLE>              font style (regular, bold, italic, etc.). Overridden by highlight [default: regular] [possible values: thin, extralight, light, regular, medium, semibold, bold, extrabold, black, italic]
      --space <SPACE>              letter spacing (in em units, e.g., 0.1) [default: 0]
      --features <FEATURES>        font features (e.g., "cv01=1,calt=0,liga=1")
      --highlight                  Enable syntax highlighting mode for files
      --theme <THEME>              Syntax highlighting theme name or path to .tmTheme file [default: base16-ocean.dark]
      --list-syntax                List supported file types/syntax for highlighting
      --list-theme                 List available built-in highlighting themes
  -d, --debug                      Enable debug logging
      --list-fonts                 List installed font families
  -h, --help                       Print help
  -V, --version                    Print version
```

## Features

- 🎨 **Text to SVG Conversion**: Convert plain text or files to SVG format
- 🎯 **Font Customization**: Support for various font families, sizes, and styles
- 📝 **Syntax Highlighting**: Built-in syntax highlighting for code files
- 🎭 **Animation Effects**: Progressive line-by-line drawing animation
- 📏 **Text Wrapping**: Support for character-based and pixel-based text wrapping
- 🎨 **Advanced Typography**: Font features, letter spacing, and style controls

## Animation Effect

The `--animate` flag creates a progressive line-by-line drawing animation where:
- Each line draws from left to right using stroke-dasharray animation
- Lines appear sequentially with a 0.8-second delay between each line
- Each line takes 1.5 seconds to complete its drawing animation
- Works best with stroke-only styling (no fill)

## Examples

### Basic text conversion
```bash
text2svg "Hello World" --font "Arial" --size 48 --output hello.svg
```

### Animated text with stroke
```bash
text2svg "Multi-line\nText Animation" --font "Arial" --animate --fill none --color "#000" --output animated.svg
```

### File with syntax highlighting
```bash
text2svg --file script.js --highlight --theme "base16-ocean.dark" --output code.svg
```

### Text wrapping by pixel width
```bash
text2svg "Long text that needs wrapping" --pixel-width 300 --font "Arial" --output wrapped.svg
```

