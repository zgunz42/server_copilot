use std::sync::{Arc, RwLock};

use actix_web::{middleware, App, HttpServer};

use crate::{controller, context};

pub async fn warp_server(
    ctxt: Arc<RwLock<context::Context>>,
) -> () {



   match HttpServer::new(move || {
        App::new()
            .app_data(
                actix_web::web::Data::new(
                    ctxt.clone()
                )
            )
            .wrap(middleware::Logger::default())
            .service(controller::index)
    })
    .bind(("127.0.0.1", 8077)) {
        Ok(s) => {
            match s.run().await {
                Ok(rn) => rn,
                Err(e) => panic!("server")
            }
        }, 
        Err(e) => panic!("server vault")
    }
}