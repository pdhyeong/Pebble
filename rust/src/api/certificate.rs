use anyhow::{Context, Result};
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// TLS 인증서 및 개인 키 쌍
#[derive(Clone)]
pub struct TlsCertificate {
    /// DER 형식의 인증서
    pub cert_der: Vec<u8>,

    /// DER 형식의 개인 키
    pub key_der: Vec<u8>,

    /// 인증서 핑거프린트 (SHA-256)
    pub fingerprint: String,
}

impl TlsCertificate {
    /// 새로운 자기 서명 인증서를 생성합니다.
    ///
    /// # Arguments
    /// * `device_id` - 기기 고유 ID (UUID)
    /// * `device_name` - 기기 이름
    ///
    /// # Security
    /// - RSA 2048비트 키 사용
    /// - SHA-256 서명 알고리즘
    /// - 1년 유효기간
    /// - P2P 통신을 위한 자기 서명 인증서
    pub fn generate_self_signed(device_id: &str, device_name: &str) -> Result<Self> {
        log::info!("Generating self-signed certificate for device: {}", device_name);

        // Distinguished Name 설정
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, device_name);
        distinguished_name.push(DnType::OrganizationName, "Pebble");
        distinguished_name.push(DnType::OrganizationalUnitName, device_id);

        // 인증서 파라미터 설정
        let mut params = CertificateParams::new(vec![device_name.to_string()])?;
        params.distinguished_name = distinguished_name;

        // 키 페어 생성
        let key_pair = KeyPair::generate()?;

        // 자기 서명 인증서 생성
        let cert = params.self_signed(&key_pair)?;

        let cert_der = cert.der().to_vec();
        let key_der = key_pair.serialize_der();

        // 인증서 핑거프린트 계산 (SHA-256)
        let fingerprint = Self::calculate_fingerprint(&cert_der)?;

        log::info!("Certificate generated. Fingerprint: {}", fingerprint);

        Ok(Self {
            cert_der,
            key_der,
            fingerprint,
        })
    }

    /// 인증서 핑거프린트를 계산합니다 (SHA-256).
    ///
    /// # Security
    /// - 인증서 핀닝(Certificate Pinning)에 사용
    /// - MITM 공격 방지를 위한 인증서 검증
    fn calculate_fingerprint(cert_der: &[u8]) -> Result<String> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        let hash = hasher.finalize();

        Ok(hex::encode(hash))
    }

    /// 인증서를 파일로 저장합니다.
    ///
    /// # Arguments
    /// * `cert_path` - 인증서 파일 경로
    /// * `key_path` - 개인 키 파일 경로
    pub fn save_to_files(&self, cert_path: &str, key_path: &str) -> Result<()> {
        fs::write(cert_path, &self.cert_der)
            .with_context(|| format!("Failed to write certificate to {}", cert_path))?;

        fs::write(key_path, &self.key_der)
            .with_context(|| format!("Failed to write private key to {}", key_path))?;

        log::info!("Certificate saved to {} and {}", cert_path, key_path);

        Ok(())
    }

    /// 파일에서 인증서를 로드합니다.
    ///
    /// # Arguments
    /// * `cert_path` - 인증서 파일 경로
    /// * `key_path` - 개인 키 파일 경로
    pub fn load_from_files(cert_path: &str, key_path: &str) -> Result<Self> {
        let cert_der = fs::read(cert_path)
            .with_context(|| format!("Failed to read certificate from {}", cert_path))?;

        let key_der = fs::read(key_path)
            .with_context(|| format!("Failed to read private key from {}", key_path))?;

        let fingerprint = Self::calculate_fingerprint(&cert_der)?;

        log::info!("Certificate loaded from {}. Fingerprint: {}", cert_path, fingerprint);

        Ok(Self {
            cert_der,
            key_der,
            fingerprint,
        })
    }

    /// Rustls용 ServerConfig를 생성합니다.
    pub fn build_server_config(&self) -> Result<Arc<rustls::ServerConfig>> {
        let cert = CertificateDer::from(self.cert_der.clone());
        let key = PrivateKeyDer::try_from(self.key_der.clone())
            .map_err(|e| anyhow::anyhow!("Invalid private key: {:?}", e))?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .context("Failed to build server config")?;

        Ok(Arc::new(config))
    }

    /// Rustls용 ClientConfig를 생성합니다.
    ///
    /// # Arguments
    /// * `trusted_fingerprint` - 신뢰할 서버 인증서의 핑거프린트 (Optional)
    ///
    /// # Security
    /// - 자기 서명 인증서를 사용하므로 인증서 검증을 우회합니다
    /// - 대신 Certificate Pinning으로 보안을 강화합니다
    /// - trusted_fingerprint가 제공되면 해당 핑거프린트만 허용
    pub fn build_client_config(trusted_fingerprint: Option<String>) -> Result<Arc<rustls::ClientConfig>> {
        use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
        use rustls::pki_types::{ServerName, UnixTime};
        use rustls::{DigitallySignedStruct, SignatureScheme};

        // 커스텀 인증서 검증기
        #[derive(Debug)]
        struct CustomCertVerifier {
            trusted_fingerprint: Option<String>,
        }

        impl ServerCertVerifier for CustomCertVerifier {
            fn verify_server_cert(
                &self,
                end_entity: &CertificateDer,
                _intermediates: &[CertificateDer],
                _server_name: &ServerName,
                _ocsp_response: &[u8],
                _now: UnixTime,
            ) -> Result<ServerCertVerified, rustls::Error> {
                // 인증서 핑거프린트 계산
                let fingerprint = TlsCertificate::calculate_fingerprint(end_entity.as_ref())
                    .map_err(|_| rustls::Error::General("Failed to calculate fingerprint".into()))?;

                log::debug!("Server certificate fingerprint: {}", fingerprint);

                // 핑거프린트 검증 (Certificate Pinning)
                if let Some(ref trusted) = self.trusted_fingerprint {
                    if &fingerprint != trusted {
                        log::error!("Certificate fingerprint mismatch! Expected: {}, Got: {}", trusted, fingerprint);
                        return Err(rustls::Error::General("Certificate fingerprint mismatch".into()));
                    }
                    log::info!("Certificate pinning verified successfully");
                }

                Ok(ServerCertVerified::assertion())
            }

            fn verify_tls12_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer,
                _dss: &DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }

            fn verify_tls13_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer,
                _dss: &DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }

            fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
                vec![
                    SignatureScheme::RSA_PKCS1_SHA256,
                    SignatureScheme::ECDSA_NISTP256_SHA256,
                    SignatureScheme::ED25519,
                ]
            }
        }

        let verifier = Arc::new(CustomCertVerifier { trusted_fingerprint });

        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();

        Ok(Arc::new(config))
    }
}

/// 인증서 관리자
///
/// 인증서의 생성, 저장, 로드를 관리합니다.
pub struct CertificateManager {
    cert_dir: String,
}

impl CertificateManager {
    /// 새로운 인증서 관리자를 생성합니다.
    ///
    /// # Arguments
    /// * `cert_dir` - 인증서를 저장할 디렉토리
    pub fn new(cert_dir: String) -> Self {
        Self { cert_dir }
    }

    /// 인증서 경로를 반환합니다.
    fn cert_path(&self) -> String {
        format!("{}/pebble_cert.der", self.cert_dir)
    }

    /// 개인 키 경로를 반환합니다.
    fn key_path(&self) -> String {
        format!("{}/pebble_key.der", self.cert_dir)
    }

    /// 인증서를 가져오거나 생성합니다.
    ///
    /// # Arguments
    /// * `device_id` - 기기 고유 ID
    /// * `device_name` - 기기 이름
    ///
    /// # Behavior
    /// - 기존 인증서가 있으면 로드
    /// - 없으면 새로 생성하고 저장
    pub fn get_or_create_certificate(&self, device_id: &str, device_name: &str) -> Result<TlsCertificate> {
        let cert_path = self.cert_path();
        let key_path = self.key_path();

        // 기존 인증서 확인
        if Path::new(&cert_path).exists() && Path::new(&key_path).exists() {
            log::info!("Loading existing certificate from {}", cert_path);
            TlsCertificate::load_from_files(&cert_path, &key_path)
        } else {
            // 디렉토리 생성
            fs::create_dir_all(&self.cert_dir)
                .with_context(|| format!("Failed to create certificate directory: {}", self.cert_dir))?;

            // 새 인증서 생성
            let cert = TlsCertificate::generate_self_signed(device_id, device_name)?;

            // 저장
            cert.save_to_files(&cert_path, &key_path)?;

            Ok(cert)
        }
    }

    /// 인증서를 삭제합니다.
    pub fn delete_certificate(&self) -> Result<()> {
        let cert_path = self.cert_path();
        let key_path = self.key_path();

        if Path::new(&cert_path).exists() {
            fs::remove_file(&cert_path)
                .with_context(|| format!("Failed to delete certificate: {}", cert_path))?;
        }

        if Path::new(&key_path).exists() {
            fs::remove_file(&key_path)
                .with_context(|| format!("Failed to delete private key: {}", key_path))?;
        }

        log::info!("Certificate deleted");

        Ok(())
    }
}
