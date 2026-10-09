// ============================================================================
// Advanced Rhai Engine: Tactical RPG State & Combat Simulation Script
// Language: Rhai (Embedded scripting language for Rust)
// ============================================================================

// --- Constants and Global Configuration ---
const CONFIG_MAX_LEVEL = 100;
const CONFIG_BASE_EXP = 1000.0;
const CONFIG_CRIT_MULTIPLIER = 1.5;
const CONFIG_MAX_INVENTORY_SLOTS = 20;

// --- Utility Functions ---
fn calculate_level_threshold(level) {
    if level >= global::CONFIG_MAX_LEVEL {
        return 999999.0;
    }
    return global::CONFIG_BASE_EXP * (level * 1.25);
}

fn clamp(val, min_val, max_val) {
    if val < min_val { return min_val; }
    if val > max_val { return max_val; }
    return val;
}

// --- Item Factory & Inventory System ---
fn create_item(name, item_type, rarity, value, stats) {
    #{
        name: name,
        item_type: item_type, // "weapon", "armor", "potion", "misc"
        rarity: rarity,       // "common", "rare", "epic", "legendary"
        value: value,
        stats: stats,         // Map of bonus stats
        equipped: false
    }
}

fn inventory_new() {
    #{
        slots: [],
        gold: 100,
        capacity: global::CONFIG_MAX_INVENTORY_SLOTS
    }
}

fn inventory_add(inv, item) {
    if inv.slots.len() >= inv.capacity {
        print("Inventory is full! Cannot add: " + item.name);
        return false;
    }
    inv.slots.push(item);
    print("Added " + item.name + " to inventory.");
    return true;
}

fn inventory_sell(inv, index) {
    if index < 0 || index >= inv.slots.len() {
        print("Invalid inventory slot index.");
        return 0;
    }
    let item = inv.slots.remove(index);
    let sale_price = item.value;
    inv.gold += sale_price;
    print("Sold " + item.name + " for " + sale_price + " gold.");
    return sale_price;
}

// --- Character / Entity System ---
fn create_character(name, char_class, base_hp, base_mp, base_atk, base_def) {
    #{
        name: name,
        class: char_class,
        level: 1,
        exp: 0.0,
        max_hp: base_hp,
        hp: base_hp,
        max_mp: base_mp,
        mp: base_mp,
        atk: base_atk,
        def: base_def,
        inventory: inventory_new(),
        equipment: #{
            weapon: (),
            armor: ()
        },
        skills: []
    }
}

fn character_gain_exp(ch, amount) {
    ch.exp += amount;
    print(ch.name + " gained " + amount + " EXP.");
    
    let threshold = calculate_level_threshold(ch.level);
    while ch.exp >= threshold && ch.level < global::CONFIG_MAX_LEVEL {
        ch.exp -= threshold;
        ch.level += 1;
        ch.max_hp += 20;
        ch.hp = ch.max_hp;
        ch.max_mp += 10;
        ch.mp = ch.max_mp;
        ch.atk += 5;
        ch.def += 3;
        print("LEVEL UP! " + ch.name + " reached level " + ch.level + "!");
        threshold = calculate_level_threshold(ch.level);
    }
}

fn character_equip(ch, inv_index) {
    if inv_index < 0 || inv_index >= ch.inventory.slots.len() {
        print("Invalid inventory index for equipping.");
        return;
    }
    
    let item = ch.inventory.slots[inv_index];
    if item.item_type != "weapon" && item.item_type != "armor" {
        print("Item " + item.name + " cannot be equipped.");
        return;
    }
    
    // Remove from inventory
    ch.inventory.slots.remove(inv_index);
    
    if item.item_type == "weapon" {
        if type_of(ch.equipment.weapon) != "()" {
            inventory_add(ch.inventory, ch.equipment.weapon);
        }
        ch.equipment.weapon = item;
        print(ch.name + " equipped weapon: " + item.name);
    } else if item.item_type == "armor" {
        if type_of(ch.equipment.armor) != "()" {
            inventory_add(ch.inventory, ch.equipment.armor);
        }
        ch.equipment.armor = item;
        print(ch.name + " equipped armor: " + item.name);
    }
}

fn character_get_total_atk(ch) {
    let bonus = 0;
    if type_of(ch.equipment.weapon) != "()" {
        if ch.equipment.weapon.stats.contains("atk") {
            bonus += ch.equipment.weapon.stats.atk;
        }
    }
    return ch.atk + bonus;
}

fn character_get_total_def(ch) {
    let bonus = 0;
    if type_of(ch.equipment.armor) != "()" {
        if ch.equipment.armor.stats.contains("def") {
            bonus += ch.equipment.armor.stats.def;
        }
    }
    return ch.def + bonus;
}

// --- Combat & Skill System ---
fn create_skill(name, mp_cost, damage_multiplier, element) {
    #{
        name: name,
        mp_cost: mp_cost,
        damage_multiplier: damage_multiplier,
        element: element
    }
}

fn combat_calculate_damage(attacker, defender, skill) {
    let base_atk = character_get_total_atk(attacker);
    let def = character_get_total_def(defender);
    
    let raw_damage = (base_atk * skill.damage_multiplier) - (def * 0.5);
    if raw_damage < 1.0 {
        raw_damage = 1.0;
    }
    
    let is_crit = (attacker.level % 2 == 0);
    if is_crit {
        raw_damage *= global::CONFIG_CRIT_MULTIPLIER;
        print("CRITICAL HIT!");
    }
    
    return int(raw_damage);
}

fn combat_execute_turn(attacker, defender, skill_index) {
    if attacker.skills.len() == 0 {
        print(attacker.name + " has no skills available!");
        return false;
    }
    
    if skill_index < 0 || skill_index >= attacker.skills.len() {
        print("Invalid skill index selected.");
        return false;
    }
    
    let skill = attacker.skills[skill_index];
    if attacker.mp < skill.mp_cost {
        print(attacker.name + " does not have enough MP to cast " + skill.name + "!");
        return false;
    }
    
    attacker.mp -= skill.mp_cost;
    print(attacker.name + " uses " + skill.name + " against " + defender.name + "!");
    
    let dmg = combat_calculate_damage(attacker, defender, skill);
    defender.hp -= dmg;
    if defender.hp < 0 {
        defender.hp = 0;
    }
    
    print(defender.name + " takes " + dmg + " damage! Remaining HP: " + defender.hp + "/" + defender.max_hp);
    
    if defender.hp == 0 {
        print(defender.name + " has been defeated!");
        return true;
    }
    
    return false;
}

// --- Quest & Event System ---
fn create_quest(title, description, reward_exp, reward_gold, required_level) {
    #{
        title: title,
        description: description,
        reward_exp: reward_exp,
        reward_gold: reward_gold,
        required_level: required_level,
        completed: false
    }
}

fn quest_complete(quest, character) {
    if quest.completed {
        print("Quest '" + quest.title + "' is already completed.");
        return false;
    }
    
    if character.level < quest.required_level {
        print("Character level too low to complete quest. Required: " + quest.required_level);
        return false;
    }
    
    quest.completed = true;
    character.inventory.gold += quest.reward_gold;
    print("QUEST COMPLETED: " + quest.title);
    print("Rewards received: " + quest.reward_gold + " gold, " + quest.reward_exp + " EXP.");
    character_gain_exp(character, quest.reward_exp);
    return true;
}

// --- Simulation Initialization and Execution Run ---
print("==================================================");
print("Initializing Tactical RPG Simulation Engine...");
print("==================================================");

let hero = create_character("Valeria", "Paladin", 150, 50, 30, 25);
let enemy = create_character("Goblin Warlord", "Monster", 200, 20, 25, 15);

let slash = create_skill("Heavy Slash", 10, 1.5, "physical");
let holy_strike = create_skill("Holy Smite", 20, 2.2, "holy");
hero.skills.push(slash);
hero.skills.push(holy_strike);

let goblin_bash = create_skill("Club Smash", 5, 1.2, "physical");
enemy.skills.push(goblin_bash);

let excalibur = create_item("Excalibur Replica", "weapon", "epic", 500, #{ atk: 25 });
let plate_mail = create_item("Steel Cuirass", "armor", "rare", 300, #{ def: 18 });
let health_potion = create_item("Greater Health Potion", "potion", "common", 50, #{ heal: 100 });

inventory_add(hero.inventory, excalibur);
inventory_add(hero.inventory, plate_mail);
inventory_add(hero.inventory, health_potion);

print("--- Equipping Gear ---");
character_equip(hero, 0);
character_equip(hero, 0);

print("Hero Stats after Equipment:");
print("ATK: " + character_get_total_atk(hero));
print("DEF: " + character_get_total_def(hero));

let q1 = create_quest("Clear the Goblin Cave", "Defeat the warlord threatening the village.", 1500.0, 250, 1);

print("--- Starting Combat Simulation ---");
let defeated = false;
let turn_count = 1;

while !defeated && turn_count <= 5 {
    print("--- Turn " + turn_count + " ---");
    defeated = combat_execute_turn(hero, enemy, 1);
    if defeated { break; }
    
    combat_execute_turn(enemy, hero, 0);
    if hero.hp <= 0 {
        print("Hero Valeria has fallen in battle!");
        break;
    }
    
    turn_count += 1;
}

if enemy.hp <= 0 {
    quest_complete(q1, hero);
}

print("==================================================");
print("Simulation Finished Successfully.");
print("Final Hero Level: " + hero.level);
print("Final Hero EXP: " + hero.exp);
print("Final Hero Gold: " + hero.inventory.gold);
print("==================================================");
