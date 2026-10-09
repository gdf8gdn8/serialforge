#[cfg(test)]
mod tests {
    use rhai::{
        AST,
        Engine,
        Map,
        Scope,
    };

    // Helper to load and compile the tactical RPG script
    fn setup_engine_and_ast() -> (Engine, AST) {
        let engine = Engine::new();
        // Assumes tactical_rpg.rhai is placed in the project root or src/
        let script_content = include_str!("../scripts/tactical_rpg.rhai");
        let ast = engine
            .compile(script_content)
            .expect("Failed to compile tactical_rpg.rhai");
        (engine, ast)
    }

    #[test]
    fn test_calculate_level_threshold() {
        let (engine, ast) = setup_engine_and_ast();
        let mut scope = Scope::new();

        // Note: Single-argument functions in Rhai require a trailing comma in the tuple argument
        let threshold_lvl1: f64 = engine
            .call_fn(&mut scope, &ast, "calculate_level_threshold", (1_i64,))
            .expect("Failed to call calculate_level_threshold for level 1");

        assert_eq!(threshold_lvl1, 1250.0); // 1000.0 * (1 * 1.25)

        let threshold_max: f64 = engine
            .call_fn(&mut scope, &ast, "calculate_level_threshold", (100_i64,))
            .expect("Failed to call calculate_level_threshold for max level");

        assert_eq!(threshold_max, 999999.0);
    }

    #[test]
    fn test_clamp_utility() {
        let (engine, ast) = setup_engine_and_ast();
        let mut scope = Scope::new();

        // Test upper bound clamp
        let clamped_high: i64 = engine
            .call_fn(&mut scope, &ast, "clamp", (15_i64, 0_i64, 10_i64))
            .expect("Failed to call clamp");
        assert_eq!(clamped_high, 10);

        // Test lower bound clamp
        let clamped_low: i64 = engine
            .call_fn(&mut scope, &ast, "clamp", (-5_i64, 0_i64, 10_i64))
            .expect("Failed to call clamp");
        assert_eq!(clamped_low, 0);

        // Test within bounds
        let normal: i64 = engine
            .call_fn(&mut scope, &ast, "clamp", (5_i64, 0_i64, 10_i64))
            .expect("Failed to call clamp");
        assert_eq!(normal, 5);
    }

    #[test]
    fn test_item_factory() {
        let (engine, ast) = setup_engine_and_ast();
        let mut scope = Scope::new();

        let stats = Map::new();
        let item: Map = engine
            .call_fn(
                &mut scope,
                &ast,
                "create_item",
                (
                    "Iron Sword".to_string(),
                    "weapon".to_string(),
                    "common".to_string(),
                    100_i64,
                    stats,
                ),
            )
            .expect("Failed to create item");

        assert_eq!(
            item.get("name").unwrap().clone().into_string().unwrap(),
            "Iron Sword"
        );
        assert_eq!(
            item.get("item_type")
                .unwrap()
                .clone()
                .into_string()
                .unwrap(),
            "weapon"
        );
        assert_eq!(item.get("value").unwrap().as_int().unwrap(), 100);
        assert_eq!(item.get("equipped").unwrap().as_bool().unwrap(), false);
    }
}
