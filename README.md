## What it do
TL;DR: Scrape through Yelp reviews for keywords, count em up, get a list.

Search for something on Yelp, you will get a URL with all of your search paramaters (`...yelp.com/search?...`). This will start there, grab all of the links to the individual business pages on Yelp, then go to those pages and search the reviews for different keywords. It'll will count how many reviews pop up for each keyword. *I know* that this isn't a perfect count but as far as I can tell, it searches for reviews that contain your whole query. If I *could* do one query that `OR`s my keywords, I *would*. But as it stands, I can't, so this has to make *a lot* of HTTP requests.

## Motivation
Combing through Yelp to find nonprofit-friendly businesses to partner with is a pain in the ass. Also, trying to learn Rust.

## Bottlenecks
Make your time tradeoffs wisely when choosing your initial query and keywords:
* Pagination: one HTTP request per page of results. Yelp sets 10 items per page, I set `rpp=40` to get 40, max 1000 businesses are returned -> up to 25 requests to just get the business links
* Each business page gets a request -> up to 1000
* Yelp review queries: as far as I can tell, Yelp doesn't support anything more than basic `contains` queries. So I can't search for reviews that contain `fundraise OR charity`. Have to do each individually -> <num_businesses> x <num_keywords> requests

## Resources
* [This](https://codeburst.io/web-scraping-in-rust-881b534a60f7) got me off the ground

## Directions
The URL is **required**. The other arguments and their defaults are listed below. *Note*: URL has to be wrapped in quotation marks and if you want multiword keywords, wrap it in quotation marks (e.g. `--keywords=fundraise,"bob ross",charity`)
```
cargo run -- --url="<some yelp url you get when you search for something>" --keywords=fundraise,charity --out=./out.txt
```
Badabing, badaboom

<hr>

### v0.0.1 MVP:
- [x] comb through all of the index links for a given query
- [x] scrape yelp reviews
- [x] print to txt file


### v0.1.0 MVP:
- [x] accept arguments at run time: initial url, output path, keywords
