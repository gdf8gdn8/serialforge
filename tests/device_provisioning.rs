#[cfg(test)]
mod tests {
    use rhai::{
        AST,
        Engine,
        Map,
        Scope,
    };

    fn setup_test_engine() -> (Engine, AST) {
        let engine = Engine::new();
        let script = include_str!("../scripts/device_provisioning_1.rhai");
        let ast = engine
            .compile(script)
            .expect("Failed to compile provisioning script");
        (engine, ast)
    }

    #[test]
    fn test_device_profile_resolution() {
        let (engine, ast) = setup_test_engine();
        let mut scope = Scope::new();

        let profile: Map = engine
            .call_fn(
                &mut scope,
                &ast,
                "get_device_profile",
                ("sensor_node_v1".to_string(),),
            )
            .expect("Failed to get profile");

        assert_eq!(
            profile
                .get("protocol")
                .unwrap()
                .clone()
                .into_string()
                .unwrap(),
            "mqtt"
        );
        assert_eq!(profile.get("port").unwrap().as_int().unwrap(), 8883);
    }

    #[test]
    fn test_device_config_validation() {
        let (engine, ast) = setup_test_engine();
        let mut scope = Scope::new();

        let mut valid_config = Map::new();
        valid_config.insert("device_id".into(), "dev_test".into());
        valid_config.insert("mac_address".into(), "123456789012".into());

        let is_valid: bool = engine
            .call_fn(&mut scope, &ast, "validate_device_config", (valid_config,))
            .expect("Failed validation call");

        assert!(is_valid);

        let mut invalid_config = Map::new();
        invalid_config.insert("device_id".into(), "dev_test".into());
        // Missing mac_address or too short

        let is_invalid: bool = engine
            .call_fn(
                &mut scope,
                &ast,
                "validate_device_config",
                (invalid_config,),
            )
            .expect("Failed validation call");

        assert!(!is_invalid);
    }

    #[test]
    fn test_response_evaluation() {
        let (engine, ast) = setup_test_engine();
        let mut scope = Scope::new();

        let result: Map = engine
            .call_fn(
                &mut scope,
                &ast,
                "evaluate_provisioning_response",
                (200_i64, "Success".to_string()),
            )
            .expect("Failed evaluation call");

        assert_eq!(result.get("success").unwrap().as_bool().unwrap(), true);
        assert_eq!(
            result.get("action").unwrap().clone().into_string().unwrap(),
            "activate"
        );
    }
}
