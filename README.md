the grim scraper...in RUST!

## Motivation
Combing through Yelp to find nonprofit-friendly businesses to partner with is a pain in the ass. Also, trying to learn Rust.

## Resources
* https://codeburst.io/web-scraping-in-rust-881b534a60f7


## Description
Search for something on Yelp. The URL you get on the resulting index page gets used as the initial URL this starts scraping from. Then it will:
1. Collect all of the links for those business's yelp pages
2. Sums the # of reviews that contain my keywords
3. Makes a table and prints it to a text file

## Directions
The URL is **required**. The other arguments and their defaults are listed below. *Note*: URL has to be wrapped in quotation marks and if you want multiword keywords, wrap it in quotation marks (e.g. `--keywords=fundraise,"bob ross",charity`)
```
cargo run -- --url="<some yelp url you get when you search for something>" --keywords=fundraise,nonprofit,charity --out=./out.txt
```

Badabing, badaboom


<hr>

### v0.0.1 MVP:
- [x] comb through all of the index links for a given query
- [x] scrape yelp reviews
- [x] print to txt file


### v0.1.0 MVP:
- [x] accept arguments at run time: initial url, output path, keywords
