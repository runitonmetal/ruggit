use ruggit::cmdline;
use ruggit::crypto::PasswdProtectedFile;
use ruggit::gapi;
use ruggit::gitlab_cache::CachedResources;
use ruggit::token::TokenStore;
use ruggit::uri_meta;
use std::env;

#[tokio::main]
async fn main() {
    let Some(home) = env::vars().find(|(k, _)| k == "HOME") else {
        println!("no home-path in env");
        std::process::exit(0);
    };
    let home = home.1;
    let config_path = std::path::PathBuf::from(&home).join(".config/ruggit");

    if !config_path.exists() {
        if let Err(e) = std::fs::create_dir_all(&config_path) {
            println!("unable to create config path: {e}");
        }
    }

    let passphrase = cmdline::hidden_input_with_prompt("passphrase: ").unwrap();
    let config_file = PasswdProtectedFile::new(&passphrase, config_path.join("tokens"));

    let mut tstore = TokenStore::new(config_file);

    let source = cmdline::parse_source(&env::args().nth(1).unwrap());
    let urimeta = uri_meta::UriMeta::new(&source).unwrap();

    let resource_file = PasswdProtectedFile::new(&passphrase, config_path.join("resources"));
    let mut cache = CachedResources::new(resource_file);
    let identifier = &urimeta.identifier;
    if !cache.list().contains(identifier) {
        let token = 'a: {
            if let Some(token) = tstore.get(&urimeta.domain) {
                break 'a token;
            }
            let org = cmdline::input_with_prompt("domain: ").unwrap();
            let token = cmdline::hidden_input_with_prompt("token: ").unwrap();
            tstore.add_token(&org, &token).unwrap();
            token
        };

        let gclient = gapi::GApi::new(&urimeta.domain, &token);
        let resource = gclient.resource_from_uri(&urimeta).await.unwrap();
        let variables = resource.variables().await.unwrap();
        cache.insert(&resource.meta, &variables);
    }
    let resource = cache.get(identifier).unwrap();
    println!("{}", serde_json::to_string(&resource).unwrap());
}
