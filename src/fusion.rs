// helper methods to format yelp's web query string into the api query string
extern crate reqwest;

use std::collections::HashMap;
use std::cmp::min;
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::Url;


pub fn api_ify(original: &str) -> String {
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
    if query_map.contains_key("attributes") {
        let attrs_str = query_map.get_mut("attributes").unwrap();
        *attrs_str = attrs_str.split(',').collect::<Vec<&str>>().join("");
    }
    // need new scope because of borrow checker
    if query_map.contains_key("attributes") && query_map["attributes"].is_empty() {
        query_map.remove("attributes");
    }
    let prices_str = prices.join(",");
    query_map.insert("price".to_owned(), prices_str);


    if query_map.contains_key("open_at") {
        let web_timestamp_str = query_map.get_mut("open_at").unwrap();
        let web_timestamp = web_timestamp_str.parse::<u64>().expect("open_at: NaN");
        let timestamp = format_time(web_timestamp);
        *web_timestamp_str = timestamp.to_string();
    }

    let mut final_string = String::from("https://api.yelp.com/v3/businesses/search?");
    for (para, arg) in query_map.iter() {
        if !arg.is_empty() {
            let s = format!("&{}={}", para, arg);
            final_string.push_str(&s);
        }
    }

    final_string.replace("&l=", "&location=")
}


fn get_lat_lon_radius(corner1: (f64, f64), corner2: (f64, f64)) -> (f64, f64, i32) {
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
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = d_lat.sin().powi(2) + lat1_rad.cos() * lat2_rad.cos() * d_lon.sin().powi(2);
    // earth's radius is 6371 km
    let c = (6_371_000.0 * 2.0 * a.sqrt().atan2((1.0 - a).sqrt())).round() as i32;
    let d = min(c, 40000);  // yelp's max is 40000 m

    (lat, lon, d)
}

fn format_neighborhood(location: &str) -> String {
    let split_loc: Vec<&str> = location.rsplit(':').filter(|s| !s.is_empty()).collect();
    split_loc.join(",").to_string()
}


fn format_time(web_timestamp: u64) -> u64 {
    // web_timestamp is minutes since Monday 12:00 AM. 
    let n = SystemTime::now().duration_since(UNIX_EPOCH).expect("DING DONG").as_secs();

    // UNIX time started on a Thursday
    let one_day_secs = 24 * 60 * 60;
    let first_monday = one_day_secs * 4;
    let last_monday = n - (n % first_monday) - one_day_secs;
    let timestamp = last_monday + (web_timestamp * 60);
    timestamp
}