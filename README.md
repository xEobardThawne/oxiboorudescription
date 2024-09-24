# oxibooru

Oxibooru is an image board engine based on Szurubooru.
The backend has been entirely rewritten in Rust with a focus on performance.

WARNING: This project is still in active development and I can't make any
         stability gaurantees quite yet.

## Features

- Post content: images (JPG, PNG, BMP, GIF, WEBP) and videos (MP4, MOV, WEBM)
- Post comments
- Post notes / annotations, including arbitrary polygons
- Rich JSON REST API ([see documentation](doc/API.md))
- Token based authentication for clients
- Rich search system
- Rich privilege system
- Autocomplete in search and while editing tags
- Tag categories
- Tag suggestions
- Tag implications (adding a tag automatically adds another)
- Tag aliases
- Pools and pool categories
- Duplicate and similarity detection
- Post rating and favoriting; comment rating
- Polished UI
- Browser configurable endless paging
- Browser configurable backdrop grid for transparent images

## Installation

It is recommended that you use Docker for deployment.
[See installation instructions.](doc/INSTALL.md)

## License

[GPLv3](LICENSE.md).
