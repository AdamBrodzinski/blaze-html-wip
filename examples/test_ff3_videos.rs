use blaze_html::BlazeTemplate;
use serde_json::json;

fn main() {
    let blaze = BlazeTemplate::builder()
        .set_root_directory("test_files")
        .register_component("FF3Layout", "components/ff3_layout.html")
        .build();

    let videos: Vec<_> = (0..300)
        .map(|_| {
            json!({
                "title": "2002",
                "artist": "Anne Marie",
                "chords": "C, G, Am, Em, F",
                "link": "/tutorials/2002-guitar-lesson-by-anne-marie"
            })
        })
        .collect();

    let data = json!({
        "videos": videos
    });

    match blaze.render_page("bench/ff3_videos.html", &data) {
        Ok(result) => {
            // Count how many times "2002" appears in the output
            let count = result.matches("2002").count();
            println!("Number of '2002' occurrences: {}", count);
            println!("Expected: 300 (one per video)");
            println!("Output length: {} bytes", result.len());

            // Count the number of <a> tags with class="article"
            let article_count = result.matches(r#"class="article""#).count();
            println!("Number of article links: {}", article_count);
        }
        Err(e) => {
            println!("Error: {}", e);
            println!("\nThis might be a parser issue with @media CSS queries.");
            println!("Let me check if the issue is in the template...");
        }
    }
}
