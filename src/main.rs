#[macro_use] extern crate prettytable;
extern crate reqwest;
extern crate scraper;
extern crate clap;
extern crate indicatif;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

use clap::{Arg, App};
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use prettytable::Table;
use scraper::{Html, Selector};
use reqwest::Client;
use reqwest::header::{Authorization, Bearer};
use reqwest::Url;

fn main() {
    let started = Instant::now();
    let matches = App::new("yelp-scraper")
                    .version("0.1")
                    .about("Scrapes yelp reviews")
                    .arg(Arg::with_name("url")
                            .short("u")
                            .long("url")
                            .value_name("URL")
                            .help("The url you get when you make a search on yelp. Please wrap in quotation marks")
                            .required(true)
                            .takes_value(true))
                    .arg(Arg::with_name("output")
                            .short("o")
                            .long("output")
                            .value_name("OUTPUT PATH")
                            .help("Where you want the output file to go")
                            .required(false)
                            .takes_value(true))
                    .arg(Arg::with_name("keywords")
                            .short("k")
                            .long("keywords")
                            .value_name("KEYWORDS")
                            .help("Keywords to search for")
                            .multiple(true)
                            .value_delimiter(",")
                            .required(false)
                            .takes_value(true))
                    .arg(Arg::with_name("crawl")
                            .short("c")
                            .long("crawl")
                            .value_name("CRAWL")
                            .help("Run the much longer, web crawler function to return # of reviews with your keywords per businesses. Defaults to false")
                            .required(false)
                            .takes_value(false))
                    .get_matches();

    let url_arg = matches.value_of("url").expect("URL is required");  // domain + query
    let _start_index = url_arg.find("?").expect("Your URL did not have a search query");
    let api_url = api_ify(&url_arg);                                  // domain + api query
    println!("\n\nAPI url: {}", api_url);

    let out_path = matches.value_of("output").unwrap_or("./out.txt");

    let keywords_arg = matches.values_of("keywords");
    let keywords = match keywords_arg {
        Some(v) => v.collect::<Vec<_>>(),
        None => vec!["fundraise", "charity"],
    };

    let mut table = Table::new();
    let mut out = File::create(out_path).expect("Failed to create file");

    let client = Client::new();
    let mut yelp_business_links = vec![];

    let crawl = match matches.occurrences_of("crawl") {
        0 => false,
        _ => true,
    };
    if crawl {
        let api_query_string = &api_url[_start_index..];
        get_yelp_index_links(&client, &api_query_string, 0, &mut yelp_business_links);
        println!();
        let bar = ProgressBar::new(yelp_business_links.len() as u64);
        for link in yelp_business_links {
            let num = search_reviews(&client, &link.1, &keywords);
            table.add_row(row![link.0, link.1, num]);
            bar.inc(1);
        }
        bar.finish();
    } else {
        let parsed_url = Url::parse(&api_url).unwrap();
        let mut hash_query: HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();
        println!("{:?}\n", hash_query);

        let search_terms = hash_query.remove("term").expect("Search query must have 'term=<serach term>'");
        for keyword in &keywords {
            let mut comb_query = format!("term={}+{}", search_terms, keyword);

            // rebuild the query string
            for (key, val) in hash_query.iter() {
                comb_query.push('&');
                comb_query.push_str(key);
                comb_query.push('=');
                comb_query.push_str(val);
            }
            get_yelp_index_links(&client, &comb_query, 0, &mut yelp_business_links);
        }

        let mut counts = HashMap::new();
        for link in &yelp_business_links {
            *counts.entry(link).or_insert(0) += 1;
        }

        println!();
        let bar = ProgressBar::new(counts.len() as u64);
        for (business, count) in counts.iter() {
            table.add_row(row![business.0, business.1, count]);
            bar.inc(1);
        }
        bar.finish();
    }

    table.printstd();
    write!(out, "{} -> {} : {:?}\n", url_arg, &api_url, keywords).expect("Failed to write to file");
    table.to_csv(out).expect("Failed to write to file");
    println!("\nSearched Yelp for keywords: {:?}", keywords);
    println!("Output file at {}", out_path);
    let time_elapsed = started.elapsed();
    println!("Time elasped: {}:{:02}", time_elapsed.as_secs() / 60, time_elapsed.as_secs() % 60);
}

fn api_ify(original: &str) -> String {
    let first_of_all = original.replace("find_desc", "term")
                            .replace("find_loc", "location")
                            .replace("cflt", "categories")
                            .replace("sortby", "sort_by")
                            .replace("attrs", "attributes")
                            .replace("open_now", "open_at")  // open_now=5678 -> open_at=5678
                            .replace("open_time", "open_at")  // TODO: accept both?
                            .replace("&ns=1", "")
                            .replace("&start=0", "");
    // ignore ed_attrs: these are deselected attributes (?)

    // TODO: translate some stuff in the attributes list, camel case
    let mut attributes = HashMap::new();
    attributes.insert("OnlineMessageThisBusiness", "request_a_quote");
    attributes.insert("ActiveDeal", "deals");

    let parsed_url = Url::parse(&first_of_all).unwrap();    
    let mut query_map: HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();

    if query_map.contains_key("l") {
        query_map.remove("location");
        if query_map["l"].starts_with("g:") {
            let coordinates: Vec<f64> = query_map["l"].trim_left_matches("g:")
                .split(',')
                .map(|x| x.parse().expect("Cannot parse coordinates as floats"))
                .collect();
            let corner1 = (coordinates[0], coordinates[1]);
            let corner2 = (coordinates[2], coordinates[3]);
            let (lat, lon, radius) = get_lat_lon_radius(corner1, corner2);
            query_map.insert("latitude".to_owned(), lat.to_string());
            query_map.insert("longitude".to_owned(), lon.to_string());
            query_map.insert("radius".to_owned(), radius.to_string());
            query_map.remove("l");
        } else {
            let location = query_map.get_mut("l").unwrap();
            *location = location.replace("p:", "");
            *location = format_neighborhood(&location);
            // some borrowing/memeory safety stuff prevents me from inserting 'location'
            // probably better to use HashMap<k, RefCell<V>> in real applications but for now, just replace it in the string
        }
    }

    // price is done in a wierd way:
    // attrs=RestaurantsPriceRange2.1,RestaurantsPriceRange2.2 -> price=1,2
    let mut prices = Vec::new();
    if query_map.contains_key("attributes") {
        let attrs_str = query_map.get_mut("attributes").unwrap();
        for p in 1..5 {
            let price_str = format!("RestaurantsPriceRange2.{}", p);
            if attrs_str.contains(&price_str) {
                prices.push(p.to_string());
                *attrs_str = attrs_str.replace(&price_str, "");
            }
        }
    }

    
    let prices_str = prices.join(",");
    query_map.insert("price".to_owned(), prices_str);

    let mut final_string = String::from("https://api.yelp.com/v3/businesses/search?");
    for (para, arg) in query_map.iter() {
        if !arg.is_empty() {
            let s = format!("&{}={}", para, arg);
            final_string.push_str(&s);
        }
    }

    final_string.replace("&l=", "&location=")
}


fn get_lat_lon_radius(corner1: (f64, f64), corner2: (f64, f64)) -> (f64, f64, u32) {
    // yelp uses the NE and SW corners of the map as lonNE,latNE,lonSW,latSW
    let (lon1, lat1) = corner1;
    let (lon2, lat2) = corner2;

    // center
    let lat = (lat1 + lat2) / 2.0;
    let lon = (lon1 + lon2) / 2.0;
    
    // Haversine Formula (for short distances)
    // https://andrew.hedges.name/experiments/haversine/
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let mid_lat_rad = lat.to_radians();
    let mid_lon_rad = lon.to_radians();
    let a = mid_lat_rad.sin().powi(2) + lat1_rad.cos() * lat2_rad.cos() * mid_lon_rad.sin().powi(2);
    let c = (2.0 * a.sqrt().atan2((1.0 - a).sqrt())).round() as u32;
    let d = min(6373000 * c, 40000);  // yelp's max is 40000 m

    (lat, lon, d)
}

fn format_neighborhood(location: &str) -> String {
    let split_loc: Vec<&str> = location.rsplit(':').filter(|s| !s.is_empty()).collect();
    split_loc.join(",").to_string()
}


fn get_yelp_index_links(client: &Client, query: &str, start: u32, yelp_links: &mut Vec<(String, String)>) {
    let url = "https://api.yelp.com/v3/businesses/search?limit=50&offset=".to_owned() + &start.to_string() + "&" + query;
    println!("requesting: {}", url);
    let mut resp = client.get(&url).header(Authorization(
        Bearer {
            token: "YTlZS9bCu0CldX0lXgJjuX489zgkFgbt5qniruI1RGffZbRX2_UhtitJ1tGmTldgkJ59nTKNtY1roSwAXaDPeLJ8PjT3MvghbQgys8G2W-z_QUhrn038qsJZSbqMWnYx".to_owned()     
        }
    )).send().unwrap();

    #[derive(Debug, Deserialize)]
    struct Category {
        alias: Option<String>,
        title: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct Coordinates {
        latitude: Option<f64>,
        longitude: Option<f64>,
    }

    #[derive(Debug, Deserialize)]
    struct Location {
        city: Option<String>,
        country: Option<String>,
        address1: Option<String>,
        address2: Option<String>,
        address3: Option<String>,
        state: Option<String>,
        zip_code: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct Business {
        rating: Option<f32>,
        price: Option<String>,
        phone: Option<String>,
        id: Option<String>,
        is_closed: Option<bool>,
        categories: Vec<Category>,
        review_count: Option<u64>,
        name: Option<String>,
        url: Option<String>,
        coordinates: Coordinates,
        image_url: Option<String>,
        location: Location,
        distance: Option<f64>,
        transactions: Vec<String>
    }

    #[derive(Debug, Deserialize)]
    struct Region {
        center: Coordinates,
    }

    #[derive(Debug, Deserialize)]
    struct Error {
        code: String,
        description: String,
    }

    #[derive(Debug, Deserialize)]
    struct YelpIndex {
        total: Option<u32>,
        businesses: Option<Vec<Business>>,
        region: Option<Region>,
        error: Option<Error>,
    }

    let result: YelpIndex = resp.json().expect("Could not format yelp search into json");
    match result.error {
        Some(e) => panic!("\n{:?}", e),
        _ => (),
    }

    for business in result.businesses.unwrap() {
        let name = &business.name.unwrap();
        let original_url = business.url.unwrap();
        let url = match original_url.find('?') {
            Some(v) => original_url[..v].to_string(),
            None => original_url,
        };
        yelp_links.push((name.to_owned(), url.to_owned()));
    }

    if start + 50 <  result.total.expect("No 'total' field returned. Check your seach query") && start + 50 < 1000 {
        get_yelp_index_links(client, query, start + 50, yelp_links);
    }
}


fn search_reviews(client: &Client, yelp_business_page_url: &str, keywords: &Vec<&str>) -> u32 {
    let mut count: u32 = 0;
    for keyword in keywords.iter() {
        let url = yelp_business_page_url.to_owned() + "?q=" + keyword;
        let mut resp = client.get(&url).send().expect("Connection error: business page");
        let body = resp.text().expect("Could not get document for business page");
        let fragment = Html::parse_document(&body);

        let num_reviews_selector = Selector::parse("div.feed div.feed_filters h3.feed_search-results").expect("Not a valid css selector");
        for header in fragment.select(&num_reviews_selector) {
            let num_text_v = header.text().collect::<Vec<_>>();
            let num_text = &num_text_v[0].trim();
            if let Some(idx) = num_text.find(' ') {
                let num = &num_text[0..idx].parse().unwrap_or(0);
                count += num;
            }
        }
    }
    count
}