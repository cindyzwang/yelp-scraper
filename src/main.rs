#[macro_use]
extern crate prettytable;
extern crate reqwest;
extern crate scraper;
extern crate clap;
extern crate indicatif;

use clap::{Arg, App};
use indicatif::ProgressBar;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use prettytable::Table;
use scraper::{Html, Selector};
use reqwest::Client;


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
                            .help("Keywords to scrape reviews for")
                            .multiple(true)
                            .value_delimiter(",")
                            .required(false)
                            .takes_value(true))
                    .get_matches();
    
    let domain: &str = "https://www.yelp.com";
    let url_arg = matches.value_of("url").expect("URL is required");
    // strip off the "&ns=1"
    let url = match url_arg.rfind("&ns=1") {
        Some(v) => url_arg[..v].to_owned() + "&rpp=40",
        None => url_arg.to_owned() + "&rpp=40",
    };

    let start_index = url.find("/search?").expect("Your URL did not have a search query");
    let init_query = &url[start_index..];

    let out_path = matches.value_of("output").unwrap_or("./out.txt");

    let keywords_arg = matches.values_of("keywords");
    let keywords = match keywords_arg {
        Some(v) => v.collect::<Vec<_>>(),
        None => vec!["fundraise", "charity"],
    };

    let mut table = Table::new();
    let mut out = File::create(out_path).expect("Failed to create file");

    let mut yelp_business_links = vec![];
    let client = Client::new();
    get_yelp_index_links(&client, domain, init_query, &mut yelp_business_links);

    println!();
    let bar = ProgressBar::new(yelp_business_links.len() as u64);
    for link in yelp_business_links {
        let num = search_reviews(&client, &link.1, &keywords);
        table.add_row(row![link.0, link.1, num]);
        bar.inc(1);
    }
    bar.finish();

    table.printstd();
    write!(out, "{:?}\n", keywords).expect("Failed to write to file");
    table.to_csv(out).expect("Failed to write to file");
    println!("\nSearched Yelp reviews for keywords: {:?}", keywords);
    println!("Output file at {}", out_path);
    let time_elapsed = started.elapsed();
    println!("Time elasped: {}:{:02}", time_elapsed.as_secs() / 60, time_elapsed.as_secs() % 60);
}


fn get_yelp_index_links(client: &Client, domain: &str, query: &str, yelp_links: &mut Vec<(String, String)>) {
    let url = domain.to_owned() + query;
    let mut resp = client.get(&url).send().expect("Connection error: index page");
    assert!(resp.status().is_success());

    let body = resp.text().expect("Could not get document for index page");
    let fragment = Html::parse_document(&body);
    let businesses = Selector::parse("li.regular-search-result a.biz-name").expect("Not a valid css selector");

    for business in fragment.select(&businesses) {
        let business_name = business.text().collect::<Vec<_>>()[0].trim();

        // get the business's yelp page
        if let Some(rel_path) = business.value().attr("href") {
            if let Some(end_index) = rel_path.rfind('?') {
                let business_yelp_link = domain.to_owned() + &rel_path[0..end_index];
                yelp_links.push((business_name.to_owned(), business_yelp_link));
            }
        }
    }

    // do it again on the next page
    let next_page_selector = Selector::parse("div.search-pagination a.next.pagination-links_anchor").expect("Not a valid css selector");
    for next_page_link in fragment.select(&next_page_selector) {
        if let Some(rel) = next_page_link.value().attr("href") {
            get_yelp_index_links(client, domain, &rel, yelp_links);
        }
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