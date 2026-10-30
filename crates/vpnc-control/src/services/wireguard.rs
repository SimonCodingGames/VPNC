//! WireGuard keypairs, config generation, and QR codes.

use anyhow::Context;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use qrcode::render::unicode;
use qrcode::QrCode;
use rand::rngs::OsRng;
use vpnc_common::Secret;
use x25519_dalek::{PublicKey, StaticSecret};

/// WireGuard keypair (private key is secret).
pub struct WgKeypair {
    /// Private key (raw 32 bytes).
    pub private: Secret<[u8; 32]>,
    /// Public key (raw 32 bytes).
    pub public: [u8; 32],
}

impl WgKeypair {
    /// Generates a new keypair compatible with `wg genkey` / `wg pubkey`.
    pub fn generate() -> Self {
        let private = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&private);
        Self {
            private: Secret::new(private.to_bytes()),
            public: public.to_bytes(),
        }
    }

    /// Returns the public key as WireGuard base64.
    pub fn public_b64(&self) -> String {
        B64.encode(self.public)
    }

    /// Returns the private key as WireGuard base64.
    pub fn private_b64(&self) -> String {
        B64.encode(self.private.expose())
    }
}

/// Parameters for a classic WireGuard client config.
pub struct ClientConfParams<'a> {
    /// Client private key (base64).
    pub client_private_b64: &'a str,
    /// Client tunnel IPv4 (optional).
    pub address_v4: Option<&'a str>,
    /// Client tunnel IPv6 (optional).
    pub address_v6: Option<&'a str>,
    /// Server public key (base64).
    pub server_public_b64: &'a str,
    /// Server endpoint host:port.
    pub endpoint: &'a str,
    /// Allowed IPs for the peer.
    pub allowed_ips: &'a str,
    /// DNS servers, comma-separated (optional).
    pub dns: Option<&'a str>,
}

/// Renders a `wg-quick`-compatible `.conf` file.
pub fn render_client_conf(p: &ClientConfParams<'_>) -> String {
    let mut address = String::new();
    if let Some(v4) = p.address_v4 {
        address.push_str(v4);
    }
    if let Some(v6) = p.address_v6 {
        if !address.is_empty() {
            address.push_str(", ");
        }
        address.push_str(v6);
    }
    let mut out = String::new();
    out.push_str("[Interface]\n");
    out.push_str(&format!("PrivateKey = {}\n", p.client_private_b64));
    if !address.is_empty() {
        out.push_str(&format!("Address = {address}\n"));
    }
    if let Some(dns) = p.dns {
        out.push_str(&format!("DNS = {dns}\n"));
    }
    out.push('\n');
    out.push_str("[Peer]\n");
    out.push_str(&format!("PublicKey = {}\n", p.server_public_b64));
    out.push_str(&format!("Endpoint = {}\n", p.endpoint));
    out.push_str(&format!("AllowedIPs = {}\n", p.allowed_ips));
    out.push_str("PersistentKeepalive = 25\n");
    out
}

/// Basic structural validation similar to what `wg-quick` expects.
pub fn validate_conf(conf: &str) -> anyhow::Result<()> {
    if !conf.contains("[Interface]") {
        anyhow::bail!("missing [Interface]");
    }
    if !conf.contains("[Peer]") {
        anyhow::bail!("missing [Peer]");
    }
    if !conf.contains("PrivateKey =") {
        anyhow::bail!("missing PrivateKey");
    }
    if !conf.contains("PublicKey =") {
        anyhow::bail!("missing peer PublicKey");
    }
    if !conf.contains("Endpoint =") {
        anyhow::bail!("missing Endpoint");
    }
    Ok(())
}

/// Renders a QR code as Unicode for terminals (and embeds conf text).
pub fn conf_qr_unicode(conf: &str) -> anyhow::Result<String> {
    let code = QrCode::new(conf.as_bytes()).context("qr encode")?;
    Ok(code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build())
}

/// Renders a QR code as PNG bytes.
pub fn conf_qr_png(conf: &str) -> anyhow::Result<Vec<u8>> {
    use qrcode::render::svg;
    // Prefer PNG via image crate from QR modules.
    let code = QrCode::new(conf.as_bytes()).context("qr encode")?;
    let svg_xml = code
        .render::<svg::Color>()
        .min_dimensions(256, 256)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();
    // Fallback: return SVG bytes labeled as image/svg — callers may use PNG path later.
    // For true PNG, rasterize modules manually:
    let image = code
        .render::<image::Luma<u8>>()
        .min_dimensions(256, 256)
        .build();
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    use image::ImageEncoder;
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::L8,
        )
        .context("png encode")?;
    let _ = svg_xml;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conf_roundtrip_shape() {
        let kp = WgKeypair::generate();
        let server = WgKeypair::generate();
        let conf = render_client_conf(&ClientConfParams {
            client_private_b64: &kp.private_b64(),
            address_v4: Some("10.0.0.2/32"),
            address_v6: None,
            server_public_b64: &server.public_b64(),
            endpoint: "vpn.example.com:51820",
            allowed_ips: "0.0.0.0/0, ::/0",
            dns: Some("1.1.1.1"),
        });
        validate_conf(&conf).unwrap();
        assert!(!conf.contains(&format!("{:?}", kp.private)));
    }

    #[test]
    fn qr_png_not_empty() {
        let png = conf_qr_png("test-config").unwrap();
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
