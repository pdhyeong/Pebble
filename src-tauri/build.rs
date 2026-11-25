fn main() {
    // .env 파일에서 REGISTRATION_KEY를 읽어 컴파일 타임 상수로 주입합니다.
    // .env가 없으면 환경변수에서 직접 읽습니다 (CI/CD 환경).
    let key = load_env_var("REGISTRATION_KEY");
    println!("cargo:rustc-env=REGISTRATION_KEY={key}");

    // .env 파일이 바뀌면 재빌드 트리거
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-env-changed=REGISTRATION_KEY");

    tauri_build::build()
}

fn load_env_var(name: &str) -> String {
    // 1순위: 이미 설정된 환경변수 (CI/CD, shell export)
    if let Ok(val) = std::env::var(name) {
        return val;
    }

    // 2순위: src-tauri/.env 파일
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix(&format!("{name}=")) {
                return rest.trim().to_string();
            }
        }
    }

    // 찾지 못하면 빌드 에러
    panic!(
        "\n\n[Pebble] 빌드 오류: `{name}` 환경변수가 없습니다.\n\
         src-tauri/.env 파일을 만들고 아래 내용을 추가하세요:\n\
         {name}=<서버의 REGISTRATION_SECRET과 동일한 값>\n"
    );
}
