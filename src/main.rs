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
use prettytable::Table;
use scraper::{Html, Selector};
use reqwest::Client;


fn main() {
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
    let url_arg = matches.value_of("url").unwrap();
    // strip off the "&ns=1"
    let url = match url_arg.rfind("&ns=1") {
        Some(v) => url_arg[..v].to_owned() + "&rpp=40",
        None => url_arg.to_owned() + "&rpp=40",
    };

    let start_index = url.find("/search?").unwrap();
    let init_query = &url[start_index..];

    let out_path = matches.value_of("output").unwrap_or("./out.txt");

    let keywords_arg = matches.values_of("keywords");
    let keywords = match keywords_arg {
        Some(v) => v.collect::<Vec<_>>(),
        None => vec!["fundraise", "nonprofit", "charity"],
    };

    let mut table = Table::new();
    let mut out = File::create(out_path).unwrap();

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
    write!(out, "{:?}\n", keywords).unwrap();
    table.to_csv(out).unwrap();
    println!("\nSearched Yelp reviews for keywords: {:?}", keywords);
    println!("Output file at {}", out_path);
}


fn get_yelp_index_links(client: &Client, domain: &str, query: &str, yelp_links: &mut Vec<(String, String)>) {
    let url = domain.to_owned() + query;
    let mut resp = client.get(&url).send().unwrap();
    assert!(resp.status().is_success());

    let body = resp.text().unwrap();
    let fragment = Html::parse_document(&body);
    let businesses = Selector::parse("li.regular-search-result a.biz-name").unwrap();

    for business in fragment.select(&businesses) {
        let business_name = business.text().collect::<Vec<_>>()[0].trim();

        // get the business's yelp page
        let mut rel_path = business.value().attr("href").unwrap();
        let end_index = rel_path.rfind('?').unwrap();
        let business_yelp_link = domain.to_owned() + &rel_path[0..end_index];
        yelp_links.push((business_name.to_owned(), business_yelp_link));
    }

    // do it again on the next page
    let next_page_selector = Selector::parse("div.search-pagination a.next.pagination-links_anchor").unwrap();
    for next_page_link in fragment.select(&next_page_selector) {
        let rel = next_page_link.value().attr("href").unwrap();
        get_yelp_index_links(client, domain, &rel, yelp_links);
    }
}


fn search_reviews(client: &Client, yelp_business_page_url: &str, keywords: &Vec<&str>) -> u32 {
    let mut count: u32 = 0;
    for keyword in keywords.iter() {
        let url = yelp_business_page_url.to_owned() + "?q=" + keyword;
        let mut resp = client.get(&url).send().unwrap();
        let body = resp.text().unwrap();
        let fragment = Html::parse_document(&body);

        let num_reviews_selector = Selector::parse("div.feed div.feed_filters h3.feed_search-results").unwrap();
        for header in fragment.select(&num_reviews_selector) {
            let num_text_v = header.text().collect::<Vec<_>>();
            let num_text = &num_text_v[0].trim();
            let idx = num_text.find(' ').unwrap();
            let num = &num_text[0..idx].parse().unwrap();
            count += num;
        }
    }
    count
}