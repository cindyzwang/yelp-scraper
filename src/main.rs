extern crate reqwest;
extern crate scraper;

use scraper::{Html, Selector};
use reqwest::Client;


fn main() {
    let domain: &str = "https://www.yelp.com";
    let init_query: &str = "/search?find_desc=Food&find_loc=San+Francisco,+CA";
    let mut yelp_business_links = vec![];
    let client = Client::new();
    get_yelp_index_links(&client, domain, init_query, &mut yelp_business_links);

    for link in yelp_business_links {
        creep_on_business(&client, &link);
    }
}

fn get_yelp_index_links(client: &Client, domain: &str, query: &str, yelp_links: &mut Vec<String>) {
    let url = domain.to_owned() + query;
    let mut resp = client.get(&url).send().unwrap();
    assert!(resp.status().is_success());

    let body = resp.text().unwrap();
    let fragment = Html::parse_document(&body);
    let businesses = Selector::parse("span.indexed-biz-name > a.biz-name").unwrap();

    for business in fragment.select(&businesses) {
        // let business_name = business.text().collect::<Vec<_>>();

        // get the business's yelp page
        let mut rel_path = business.value().attr("href").unwrap();
        let end_index = rel_path.rfind('?').unwrap();
        let business_yelp_link = domain.to_owned() + &rel_path[0..end_index];
        yelp_links.push(business_yelp_link);
    }

    // do it again on the next page
    let next_page_selector = Selector::parse("div.arrange_unit > a.next.pagination-links_anchor").unwrap();
    for next_page_link in fragment.select(&next_page_selector) {
        let rel = next_page_link.value().attr("href").unwrap();
        println!("next: {}", rel);
        get_yelp_index_links(client, domain, &rel, yelp_links);
    }
}

fn creep_on_business(client: &Client, yelp_business_page_url: &str) {
    // get # of reviews with our keywords
    search_reviews(client, yelp_business_page_url);

    // let mut resp = client.get(yelp_business_page_url).send().unwrap();
    // let body = resp.text().unwrap();
    // let fragment = Html::parse_document(&body);

    // let address_selector = Selector::parse("div.mapbox-text > ul > li > span.biz-website").unwrap();
    // for addr in fragment.select(&address_selector) {
    //     // crawl
    // }
}

// fn crawl_website() {
//     // search for ["fundraise", "event"]
// }

fn search_reviews(client: &Client, yelp_business_page_url: &str) -> u32 {
    let keywords = ["fundraise", "nonprofit", "non%20profit", "non-profit", "charity"];

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