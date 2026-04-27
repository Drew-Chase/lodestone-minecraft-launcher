#[actix_web::main]
async fn main()->anyhow::Result<()>{
	lodestone_website_lib::run().await
}
