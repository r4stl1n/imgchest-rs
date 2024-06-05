# imgchest-rs
A Rust library to interact with [imgchest.com](https://imgchest.com).

## Design
This library will attempt to completely avoid official API usage.
This is because of 2 reasons:
1. The API requires a login to access public data.
2. The API has obscene ratelimits, only 60 per hour.

As a result, a design based on scraping will be superior to one based on recommended API usage.
If the API is fixed, this library will be likely be reworked to target that instead.


API objects in this library are tailored to match the official API's as much as possible, 
though some fields are missing and extra fields are included where needed.
Usage of these objects is also noticablely more complicated.
As an example, the Post object may not load all images in one API call and may need a second call.

## References
 * https://imgchest.com/docs/api/1.0/general/overview