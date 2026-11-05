//! Contrat vérifié dans les sources officielles, voir docs/FOURNISSEUR.md.
//! Aucun retry de POST. Les URL signées et les erreurs distantes ne sont jamais affichées.
use crate::{error::invalid, model::valid_remote_id, Error, ProviderPin, Result};
use reqwest::{
    blocking::{Client, Response},
    redirect::Policy,
    Url,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{io::Read, time::Duration};

pub trait SegmentationProvider {
    fn describe(&self, key: &str) -> Result<ProviderPin>;
    fn upload(&self, key: &str, png: Vec<u8>) -> Result<String>;
    fn submit(&self, key: &str, pin: &ProviderPin, uri: &str) -> Result<String>;
    fn task(&self, key: &str, id: &str) -> Result<RemoteTask>;
    fn result(&self, key: &str, id: &str, max_bytes: u64) -> Result<Vec<u8>>;
}
#[derive(Deserialize, Debug)]
pub struct RemoteTask {
    pub id: String,
    pub status: u32,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_version_id: String,
    #[serde(default)]
    pub input: Value,
}

pub struct InferenceSh {
    client: Client,
    base: String,
    #[cfg(test)]
    local_test: bool,
}
impl InferenceSh {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .retry(reqwest::retry::never())
                .redirect(Policy::none())
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(|_| Error::Network)?,
            base: "https://api.inference.sh".into(),
            #[cfg(test)]
            local_test: false,
        })
    }
    fn json(&self, response: std::result::Result<Response, reqwest::Error>) -> Result<Value> {
        let bytes = self.bytes(response, 2 * 1024 * 1024)?;
        let v: Value = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Réponse JSON du fournisseur invalide."))?;
        Ok(v.get("data").cloned().unwrap_or(v))
    }
    fn bytes(
        &self,
        response: std::result::Result<Response, reqwest::Error>,
        limit: u64,
    ) -> Result<Vec<u8>> {
        let r = response.map_err(|_| Error::Network)?;
        if !r.status().is_success() {
            return Err(Error::Http(r.status().as_u16()));
        }
        if r.content_length().is_some_and(|s| s > limit) {
            return Err(invalid("Réponse du fournisseur trop volumineuse."));
        }
        let mut out = Vec::new();
        r.take(limit + 1)
            .read_to_end(&mut out)
            .map_err(|_| Error::Network)?;
        if out.len() as u64 > limit {
            return Err(invalid("Réponse du fournisseur trop volumineuse."));
        }
        Ok(out)
    }
    pub fn validate_transfer_url(&self, value: &str) -> Result<Url> {
        let u = Url::parse(value).map_err(|_| invalid("URL de fichier distante invalide."))?;
        #[cfg(test)]
        if self.local_test && value.starts_with(&format!("{}/", self.base)) {
            return Ok(u);
        }
        if u.scheme() != "https"
            || u.host_str() != Some("cloud.inference.sh")
            || u.port_or_known_default() != Some(443)
            || !u.username().is_empty()
            || u.password().is_some()
            || u.fragment().is_some()
        {
            return Err(invalid("Destination de fichier non autorisée. Vérifier les domaines officiels du fournisseur avant d’étendre la liste."));
        }
        Ok(u)
    }
}
fn string(v: &Value, key: &str) -> Result<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid("Réponse du fournisseur incomplète."))
}
impl SegmentationProvider for InferenceSh {
    fn describe(&self, key: &str) -> Result<ProviderPin> {
        let app = self.json(
            self.client
                .get(format!("{}/apps/infsh/birefnet", self.base))
                .bearer_auth(key)
                .send(),
        )?;
        if app["namespace"] != "infsh" || app["name"] != "birefnet" {
            return Err(invalid(
                "Le fournisseur retourne une autre application que BiRefNet.",
            ));
        }
        let version = &app["version"];
        let function = version
            .get("default_function")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let schema = if let Some(f) = &function {
            &version["functions"][f]["input_schema"]
        } else {
            &version["input_schema"]
        };
        if schema["properties"]["image"]["type"] != "string"
            || schema["required"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v != "image"))
        {
            return Err(invalid("Le schéma BiRefNet a changé. Le connecteur doit être vérifié avant toute soumission."));
        }
        let setup = &version["setup_schema"];
        if setup["required"].as_array().is_some_and(|a| !a.is_empty()) {
            return Err(invalid(
                "Le modèle exige des réglages supplémentaires non pris en charge.",
            ));
        }
        let pin = ProviderPin {
            app_id: string(&app, "id")?,
            version_id: string(&app, "version_id")?,
            function,
        };
        pin.validate()?;
        if version.get("id").and_then(Value::as_str) != Some(pin.version_id.as_str()) {
            return Err(invalid(
                "Version du modèle non résolue ; aucune soumission autorisée.",
            ));
        }
        Ok(pin)
    }
    fn upload(&self, key: &str, png: Vec<u8>) -> Result<String> {
        let value=self.json(self.client.post(format!("{}/files",self.base)).bearer_auth(key)
            .json(&json!({"files":[{"uri":"","filename":"source.png","content_type":"image/png","size":png.len()}]})).send())?;
        let files = value
            .as_array()
            .filter(|a| a.len() == 1)
            .ok_or_else(|| invalid("Réponse d’import distante invalide."))?;
        let url = self.validate_transfer_url(&string(&files[0], "upload_url")?)?;
        // Jamais de clé API sur l’hôte de stockage.
        self.bytes(
            self.client
                .put(url)
                .header("Content-Type", "image/png")
                .body(png)
                .send(),
            64 * 1024,
        )?;
        string(&files[0], "uri")
    }
    fn submit(&self, key: &str, pin: &ProviderPin, uri: &str) -> Result<String> {
        pin.validate()?;
        let mut body = json!({"app_id":pin.app_id,"version_id":pin.version_id,"input":{"image":uri},"wait":false});
        if let Some(f) = &pin.function {
            body["function"] = json!(f);
        }
        let value = self.json(
            self.client
                .post(format!("{}/apps/run", self.base))
                .bearer_auth(key)
                .json(&body)
                .send(),
        )?;
        let id = string(&value, "id")?;
        valid_remote_id(&id)?;
        Ok(id)
    }
    fn task(&self, key: &str, id: &str) -> Result<RemoteTask> {
        valid_remote_id(id)?;
        let value = self.json(
            self.client
                .get(format!("{}/tasks/{id}", self.base))
                .bearer_auth(key)
                .send(),
        )?;
        let task: RemoteTask =
            serde_json::from_value(value).map_err(|_| invalid("État distant invalide."))?;
        if task.id != id || task.status > 13 {
            return Err(invalid(
                "Le fournisseur retourne un état ou identifiant inattendu.",
            ));
        }
        Ok(task)
    }
    fn result(&self, key: &str, id: &str, max_bytes: u64) -> Result<Vec<u8>> {
        valid_remote_id(id)?;
        // Endpoint documenté : pas d’hypothèse sur les clés JSON de output.
        let value = self.json(
            self.client
                .get(format!("{}/tasks/{id}/files", self.base))
                .query(&[("role", "output")])
                .bearer_auth(key)
                .send(),
        )?;
        let files = value
            .as_array()
            .ok_or_else(|| invalid("Liste des résultats invalide."))?;
        let images: Vec<_> = files
            .iter()
            .filter(|v| v["content_type"] == "image/png")
            .collect();
        if images.len() != 1 {
            return Err(invalid("Le résultat doit contenir exactement un fichier PNG. Vérification du connecteur nécessaire."));
        }
        let url = self.validate_transfer_url(&string(images[0], "uri")?)?;
        self.bytes(self.client.get(url).send(), max_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use tiny_http::{Response as TestResponse, Server};
    fn server(responses: Vec<(u16, Value)>) -> (InferenceSh, thread::JoinHandle<Vec<String>>) {
        let server = Server::http("127.0.0.1:0").unwrap();
        let mut p = InferenceSh::new().unwrap();
        p.base = format!("http://{}", server.server_addr());
        p.local_test = true;
        let h = thread::spawn(move || {
            responses
                .into_iter()
                .map(|(status, body)| {
                    let mut r = server
                        .recv_timeout(Duration::from_secs(5))
                        .unwrap()
                        .unwrap();
                    let mut b = String::new();
                    r.as_reader().read_to_string(&mut b).unwrap();
                    let request = format!("{} {} {}", r.method(), r.url(), b);
                    r.respond(TestResponse::from_string(body.to_string()).with_status_code(status))
                        .unwrap();
                    request
                })
                .collect()
        });
        (p, h)
    }
    #[test]
    fn documented_submit_and_poll_contract() {
        let (p, h) = server(vec![
            (200, json!({"data":{"id":"remote-1","status":1}})),
            (200, json!({"data":{"id":"remote-1","status":10}})),
        ]);
        let pin = ProviderPin {
            app_id: "app1".into(),
            version_id: "version1".into(),
            function: None,
        };
        assert_eq!(
            p.submit("test-only", &pin, "inf://files/test").unwrap(),
            "remote-1"
        );
        assert_eq!(p.task("test-only", "remote-1").unwrap().status, 10);
        let requests = h.join().unwrap();
        assert!(requests[0].starts_with("POST /apps/run"));
        assert!(requests[0].contains("\"version_id\":\"version1\""));
        assert!(requests[1].starts_with("GET /tasks/remote-1"));
    }
    #[test]
    fn refusal_is_never_retried() {
        for status in [401, 402, 429, 500, 503] {
            let (p, h) = server(vec![(status, json!({"error":"sensitive-remote-text"}))]);
            let e = p.task("test-only", "task").unwrap_err().to_string();
            assert!(!e.contains("sensitive"));
            assert_eq!(h.join().unwrap().len(), 1);
        }
    }
    #[test]
    fn urls_fail_closed() {
        let p = InferenceSh::new().unwrap();
        for url in [
            "http://cloud.inference.sh/x",
            "https://127.0.0.1/x",
            "https://cloud.inference.sh.evil.test/x",
            "https://cloud.inference.sh:444/x",
            "https://u:p@cloud.inference.sh/x",
            "https://[::1]/x",
        ] {
            assert!(p.validate_transfer_url(url).is_err(), "{url}");
        }
        assert!(p
            .validate_transfer_url("https://cloud.inference.sh/u/result.png?signature=abc")
            .is_ok());
    }
    #[test]
    fn invalid_task_and_response() {
        let (p, h) = server(vec![(200, json!({"data":{"id":"wrong","status":10}}))]);
        assert!(p.task("test-only", "task").is_err());
        h.join().unwrap();
        assert!(p.task("test-only", "../../secret").is_err());
    }
    #[test]
    fn complete_http_pipeline_without_paid_service() {
        use crate::{imaging, Category, Composition, ExportFormat, Library};
        use image::{DynamicImage, Rgba, RgbaImage};
        let server = Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr());
        let mut provider = InferenceSh::new().unwrap();
        provider.base = base.clone();
        provider.local_test = true;
        let h = thread::spawn(move || {
            let mut calls = 0;
            for step in 0..7 {
                let mut request = server
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap()
                    .unwrap();
                let mut body = Vec::new();
                request.as_reader().read_to_end(&mut body).unwrap();
                let has_auth = request
                    .headers()
                    .iter()
                    .any(|h| h.field.equiv("Authorization"));
                let (route, response) = match step {
                    0 => (
                        "/apps/infsh/birefnet",
                        json!({"data":{"id":"app1","namespace":"infsh","name":"birefnet","version_id":"version1","version":{"id":"version1","input_schema":{"properties":{"image":{"type":"string"}},"required":["image"]}}}}),
                    ),
                    1 => (
                        "/files",
                        json!({"data":[{"uri":"inf://files/source","upload_url":format!("{base}/upload")}]}),
                    ),
                    2 => {
                        assert_eq!(request.url(), "/upload");
                        assert!(!has_auth);
                        assert!(image::load_from_memory(&body).is_ok());
                        request.respond(TestResponse::empty(204)).unwrap();
                        continue;
                    }
                    3 => {
                        calls += 1;
                        let body: Value = serde_json::from_slice(&body).unwrap();
                        assert_eq!(body["version_id"], "version1");
                        assert_eq!(body["input"]["image"], "inf://files/source");
                        ("/apps/run", json!({"data":{"id":"task1","status":1}}))
                    }
                    4 => (
                        "/tasks/task1",
                        json!({"data":{"id":"task1","status":10,"app_id":"app1","app_version_id":"version1","input":{"image":"inf://files/source"}}}),
                    ),
                    5 => (
                        "/tasks/task1/files?role=output",
                        json!({"data":[{"content_type":"image/png","uri":format!("{base}/result.png")}]}),
                    ),
                    _ => {
                        assert_eq!(request.url(), "/result.png");
                        assert!(!has_auth);
                        let rgba = RgbaImage::from_fn(80, 60, |x, y| {
                            Rgba([
                                0,
                                0,
                                0,
                                if (10..70).contains(&x) && (10..50).contains(&y) {
                                    255
                                } else {
                                    0
                                },
                            ])
                        });
                        request
                            .respond(TestResponse::from_data(
                                imaging::png(&DynamicImage::ImageRgba8(rgba)).unwrap(),
                            ))
                            .unwrap();
                        continue;
                    }
                };
                assert_eq!(request.url(), route);
                assert!(has_auth);
                request
                    .respond(TestResponse::from_string(response.to_string()))
                    .unwrap();
            }
            calls
        });
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source.png");
        std::fs::write(
            &input,
            imaging::png(&DynamicImage::ImageRgba8(RgbaImage::from_pixel(
                80,
                60,
                Rgba([200, 20, 90, 255]),
            )))
            .unwrap(),
        )
        .unwrap();
        let lib = Library::open(dir.path().join("library")).unwrap();
        let photo = lib.import(&input).unwrap();
        lib.classify(&photo.id, Category::Exterior).unwrap();
        let pin = lib.pin_provider(&provider, "test-only").unwrap();
        lib.submit(&photo.id, &pin.version_id, &provider, "test-only")
            .unwrap();
        lib.synchronize(&photo.id, None, &provider, "test-only")
            .unwrap();
        lib.compose(
            &photo.id,
            &Composition {
                width: 256,
                height: 256,
                ..Composition::default()
            },
        )
        .unwrap();
        lib.review(&photo.id, true).unwrap();
        let out = lib
            .export(&photo.id, dir.path(), ExportFormat::Webp, 90)
            .unwrap();
        assert_eq!(image::open(out).unwrap().width(), 256);
        assert_eq!(h.join().unwrap(), 1);
    }
    #[test]
    fn malformed_json_and_redirects_are_not_accepted() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let mut p = InferenceSh::new().unwrap();
        p.base = format!("http://{}", server.server_addr());
        p.local_test = true;
        let h = thread::spawn(move || {
            let r = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            r.respond(TestResponse::from_string("invalid-json-sensitive-body"))
                .unwrap();
            let r = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            r.respond(TestResponse::empty(302).with_header(
                tiny_http::Header::from_bytes("Location", "http://127.0.0.1:1/unexpected").unwrap(),
            ))
            .unwrap();
        });
        let error = p.task("test-only", "task").unwrap_err().to_string();
        assert!(!error.contains("sensitive"));
        assert!(matches!(p.task("test-only", "task"), Err(Error::Http(302))));
        h.join().unwrap();
    }
    #[test]
    fn ambiguous_http_timeout_does_not_retry_submission() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let mut p = InferenceSh::new().unwrap();
        p.base = format!("http://{}", server.server_addr());
        p.local_test = true;
        p.client = Client::builder()
            .retry(reqwest::retry::never())
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();
        let h = thread::spawn(move || {
            let r = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            assert_eq!(r.method().as_str(), "POST");
            thread::sleep(Duration::from_millis(200));
            let _ = r.respond(TestResponse::from_string("{}"));
            assert!(server
                .recv_timeout(Duration::from_millis(150))
                .unwrap()
                .is_none());
        });
        assert!(matches!(
            p.submit(
                "test-only",
                &ProviderPin {
                    app_id: "app".into(),
                    version_id: "version".into(),
                    function: None
                },
                "inf://files/test"
            ),
            Err(Error::Network)
        ));
        h.join().unwrap();
    }
}
