use std::sync::Arc;

fn main() {
    let tls_connector = Arc::new(native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap());
    
    let agent = ureq::builder()
        .tls_connector(tls_connector)
        .build();
    
    println!("Builder worked!");
}
