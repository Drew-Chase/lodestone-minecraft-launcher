use actix_web::{get, web, HttpResponse, Responder};

const BASE_URL: &str = "https://lodestonelauncher.com";

#[get("/robots.txt")]
async fn robots_txt() -> impl Responder {
    let body = format!(
        "User-agent: *\n\
         Allow: /\n\
         \n\
         Sitemap: {BASE_URL}/sitemap.xml\n"
    );
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(body)
}

#[get("/sitemap.xml")]
async fn sitemap_xml() -> impl Responder {
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>{BASE_URL}/</loc>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>
"#
    );
    HttpResponse::Ok()
        .content_type("application/xml; charset=utf-8")
        .body(body)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(robots_txt)
        .service(sitemap_xml);
}
