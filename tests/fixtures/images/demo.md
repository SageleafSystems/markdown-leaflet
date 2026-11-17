# Markdown Leaflet: image demo

A longer document for trying out images next to ordinary text. Open the **repository folder** (`cargo run --locked -- .`) and pick this file from the sidebar, so the brand images below sit inside the open folder. Opening this file on its own limits images to its own folder, and the brand images will show as failures.

## Why images need care

Images in a preview have to balance three things: they should look sharp at their natural size, they must never overflow the column, and a missing file should never break the page. Here is a paragraph of plain text to give the images some context to sit between, with `inline code`, a [link to the fixture list](images.md), and *emphasis* for contrast.

> Good documentation shows the thing it describes. A screenshot at the right moment saves a paragraph of explanation.

### The brand logo

The horizontal logo is a 2200 × 600 PNG, much wider than the text column at most window sizes, so it should shrink to fit and keep its proportions:

![Markdown Leaflet horizontal logo](../../../assets/brand/markdown-leaflet-logo.png)

Resize the window, and open and close the gutters. The logo should follow the column width and never scale up past its natural size.

### The app icon

The icon is a 1024 × 1024 PNG with transparent margins. It should sit at the column width at most, with the page color showing through the transparent corners:

![Markdown Leaflet app icon](../../../assets/brand/markdown-leaflet-icon.png)

## Natural size and small images

The next image is only 48 × 48 pixels. It must stay small rather than blowing up to the column width:

![A small green checkerboard](img/small.png)

A 640 × 320 gradient follows. On a wide window it draws at exactly its natural size:

![A 640 by 320 gradient](img/gradient.png)

And an extra-wide 2400 × 600 gradient, which has to be scaled down:

![A very wide gradient](img/huge.png)

## Vector images

SVG files are drawn by the renderer at whatever size they need:

![A blue banner with shapes](img/shapes.svg)

## Images inside other blocks

Lists and quotes can hold images too:

- First, an item with only text.
- Second, an item with an image below it:

  ![A checkerboard inside a list item](img/small.png)

- Third, another plain item.

> A quote with a picture:
>
> ![A gradient inside a quote](img/gradient.png)
>
> and some trailing text after it.

## A little code and a table

Nothing about images changes how code or tables render:

```rust
fn main() {
    println!("images and code share a page");
}
```

| Case | Expected |
| --- | --- |
| Local PNG, JPEG, GIF, WebP, BMP | Drawn at natural size, shrunk to fit |
| Local SVG | Drawn by the renderer |
| Missing file | Alt text with a reason |
| Remote `https://` image | Not fetched; alt text with a reason |
| Outside the open folder | Alt text with a reason |

## Failures

Each of these shows its alt text and a short reason instead of an image:

![This file does not exist](img/does-not-exist.png)

![Remote images are never fetched](https://example.com/logo.png)

![A text file is not an image](img/notes.txt)

## Find and selection

Search for the word *gradient* with Ctrl/⌘F. The count includes the alt text of the gradient images, and the match counter should stay put as the images finish loading. Selecting across an image and copying includes its alt text.

## Known limits

- Images inside table cells are not loaded yet.
- Animated GIFs show their first frame.
- Editing only an image file does not refresh it until the Markdown file itself reloads.

![The end: the logo again, at the bottom of a long page](../../../assets/brand/markdown-leaflet-logo.png)
