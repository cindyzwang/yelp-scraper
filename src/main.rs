extern crate reqwest;
extern crate scraper;

use scraper::{Html, Selector};


fn main() {
    let domain: &str = "https://www.yelp.com";
    let init_query: &str = "/search?find_desc=Food&find_loc=San+Francisco,+CA";
    let mut yelp_business_links = vec![];
    get_yelp_index_links(domain, init_query, &mut yelp_business_links);
    // for item in links.iter() {
    //     println!("business: {}", item.to_string());
    // }

}

// keywords to search reviews for:
// ["fundraise", "nonprofit", "non%20profit", "non-profit", "charity"]

fn get_yelp_index_links(domain: &str, query: &str, yelp_links: &mut Vec<String>) {
    let url = domain.to_owned() + query;
    let mut resp = reqwest::get(&url).unwrap();
    assert!(resp.status().is_success());

    let body = resp.text().unwrap();
    let fragment = Html::parse_document(&body);
    let businesses = Selector::parse("span.indexed-biz-name > a.biz-name").unwrap();

    for business in fragment.select(&businesses) {
        // let business_name = business.text().collect::<Vec<_>>();

        // get the business's yelp page
        let mut rel_path = business.value().attr("href").unwrap();
        let business_yelp_link = url.to_owned() + rel_path;
        yelp_links.push(business_yelp_link);
    }

    let next_page_selector = Selector::parse("div.arrange_unit > a.next.pagination-links_anchor").unwrap();
    for next_page_link in fragment.select(&next_page_selector) {
        let rel = next_page_link.value().attr("href").unwrap();
        println!("next: {}", rel);
        get_yelp_index_links(domain, &rel, yelp_links);
    }
}