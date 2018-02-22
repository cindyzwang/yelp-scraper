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

    let url_arg = matches.value_of("url").expect("URL is required");
    // strip off the "&ns=1" if it is there
    let url = match url_arg.rfind("&ns=1") {
        Some(v) => url_arg[..v].to_owned(),
        None => url_arg.to_owned(),
    };

    let start_index = url.find("?").expect("Your URL did not have a search query");
    let init_query = &url[start_index + 1..].replace("find_desc", "term").replace("find_loc", "location");
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
        get_yelp_index_links(&client, init_query, 0, &mut yelp_business_links);
        println!();
        let bar = ProgressBar::new(yelp_business_links.len() as u64);
        for link in yelp_business_links {
            let num = search_reviews(&client, &link.1, &keywords);
            table.add_row(row![link.0, link.1, num]);
            bar.inc(1);
        }
        bar.finish();
    } else {
        let temp_url = "https://api.yelp.com?".to_owned() + init_query;
        let parsed_url = Url::parse(&temp_url).unwrap();
        let mut hash_query: HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();

        let search_terms = hash_query.remove("term").unwrap();
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
            if counts.contains_key(&link) {
                *counts.get_mut(link).unwrap() += 1;
            } else {
                counts.insert(link, 1);
            }
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
    write!(out, "{} : {:?}\n", init_query, keywords).expect("Failed to write to file");
    table.to_csv(out).expect("Failed to write to file");
    println!("\nSearched Yelp for keywords: {:?}", keywords);
    println!("Output file at {}", out_path);
    let time_elapsed = started.elapsed();
    println!("Time elasped: {}:{:02}", time_elapsed.as_secs() / 60, time_elapsed.as_secs() % 60);
}


fn get_yelp_index_links(client: &Client, query: &str, start: u32, yelp_links: &mut Vec<(String, String)>) {
    let url = "https://api.yelp.com/v3/businesses/search?limit=50&offset=".to_owned() + &start.to_string() + "&" + query;
    println!("{}", url);
    let mut resp = client.get(&url).header(Authorization(
        Bearer {
            token: "YTlZS9bCu0CldX0lXgJjuX489zgkFgbt5qniruI1RGffZbRX2_UhtitJ1tGmTldgkJ59nTKNtY1roSwAXaDPeLJ8PjT3MvghbQgys8G2W-z_QUhrn038qsJZSbqMWnYx".to_owned()     
        }
    )).send().unwrap();
    assert!(resp.status().is_success());

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
    struct YelpIndex {
        total: u32,
        businesses: Vec<Business>,
        region: Region,
    }

    let result: YelpIndex = resp.json().unwrap();

    for business in result.businesses {
        let name = &business.name.unwrap();
        let original_url = business.url.unwrap();
        let url = match original_url.find('?') {
            Some(v) => original_url[..v].to_string(),
            None => original_url,
        };
        yelp_links.push((name.to_owned(), url.to_owned()));
    }

    if start + 50 <  result.total && start + 50 < 1000 {
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