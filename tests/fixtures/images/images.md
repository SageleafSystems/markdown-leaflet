# Image fixture

Open this file to check block images. Run from the folder `tests/fixtures/images`.

## Raster, natural size

![A gradient, 640 by 320](img/gradient.png)

## Small image is not upscaled

![Small checkerboard](img/small.png)

## Wider than the preview shrinks to fit

![A very wide gradient](img/huge.png)

## SVG

![Blue banner](img/shapes.svg)

## Inside a list and a quote

- ![Checkerboard in a list](img/small.png)

> ![Gradient in a quote](img/gradient.png)

## Failures (each shows its alt text and a reason)

![Missing file](img/does-not-exist.png)

![Remote image is not fetched](https://example.com/logo.png)

![Outside the folder](../../../assets/brand/markdown-leaflet-icon.svg)

![Unsupported type](img/notes.txt)
