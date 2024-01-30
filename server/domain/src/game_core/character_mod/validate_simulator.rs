use atb::prelude::*;
use strum::EnumCount;

use crate::game_core::character_mod::accessory_module::AccPart;
use crate::game_core::config::{Element, GameplayConfigManager};
use crate::game_core::probability_mod::*;

// for testing
use super::accessory_module::AccessoryModule;
use super::attribute::Attribute;
use super::char_const::*;

const HP_MULTI_PART_LOG: &'static [&str] = &[
    "Has (head + face)",
    "Has (head + neck)",
    "Has (face + neck)",
];

const HP_SINGLE_PART_LOG: &'static [&str] = &["Only has head", "Only has face", "Only has neck"];

const DEF_3_BODY_PART_LOG: &'static [&str] = &[
    "Has (body + waist + arm)",
    "Has (body + waist + foot)",
    "Has (body + arm + foot)",
    "Has (waist + arm + foot)",
];

const DEF_2_BODY_PART_LOG: &'static [&str] = &[
    "Has (body + waist)",
    "Has (body + arm)",
    "Has (body + foot)",
    "Has (waist + arm)",
    "Has (waist + foot)",
];

const DEF_1_BODY_PART_LOG: &'static [&str] = &["Only has body", "Only has waist"];

const ATK_2_PART_LOG: &'static [&str] = &["Has (eyes + weapon)", "Has (eyes + sidearms)"];

const MONO_SPC_2_PART_LOG: &'static &str = &"Has only (floating + ground)";

const DUAL_SPC_4_PART_LOG: &str = "Has ([ground effect] + floating item + ground item + bg effect)";

const DUAL_SPC_3_PART_LOG: &'static [&str] = &[
    "Has ([ground effect] + floating item + ground item)",
    "Has ([ground effect] + floating item + bg effect)",
    "Has ([ground effect] + ground item + bg effect)",
];

const DUAL_SPC_2_PART_LOG: &'static [&str] = &[
    "Has ([ground effect] + floating item)",
    "Has ([ground effect] + ground item)",
    "Has ([ground effect] + bg effect)",
];

const ACCESSORY_EACH_PART_LOG: &'static [&str] = &[
    "Has head",
    "Has face",
    "Has neck",
    "Has body",
    "Has waist",
    "Has arm",
    "Has foot",
    "Has eyes",
    "Has weapon",
    "Has sidearms",
    "Has floating item 1",
    "Has ground item 1",
    "Has bg effect 1",
    "Has ground effect 2",
    "Has floating item 2",
    "Has ground item 2",
    "Has bg effect 2",
];

pub fn run_simulator(tier_lv: usize, simulation_count: u32) -> String {
    //let mut accumalate_acc: AccessoryModule = Default::default();
    let mut result_log: String = format!(
        "\n --- Tier: {}, Simulating count: {}\n",
        tier_lv, simulation_count
    );

    // -- HP accessories statistic --
    // [0]: Has only 1 accessory
    // [1]: Has only 2 accessories
    // [2]: Has all 3 accessories
    let mut hp_acc_amount_acquired = vec![0; 3];

    // [0]: (head + face)
    // [1]: (head + neck)
    // [2]: (face + neck)
    let mut hp_acc_acquired_2 = vec![0; 3];

    // [0]: Only has head
    // [1]: Only has face
    // [2]: Only has neck
    let mut hp_acc_acquired_1 = vec![0; 3];

    // -- DEF accessories statistic --
    // [0]: Has only 1 accessory
    // [1]: Has only 2 accessories
    // [2]: Has only 3 accessories
    // [3]: Has all 4 accessories
    let mut def_acc_amount_acquired = vec![0; 4];

    // [0]: Has (body + waist + arm)
    // [1]: Has (body + waist + foot)
    // [2]: Has (body + arm + foot)
    // [3]: Has (waist + arm + foot)
    let mut def_acc_body_part_acquired_3 = vec![0; 4];

    // [0]: Has (body + waist)
    // [1]: Has (body + arm)
    // [2]: Has (body + foot)
    // [3]: Has (waist + arm)
    // [4]: Has (waist + foot)
    let mut def_acc_body_part_acquired_2 = vec![0; 5];

    // [0]: Only has body
    // [1]: Only has waist
    let mut def_acc_body_part_acquired_1 = vec![0; 2];

    // Has remain single accesseory
    let mut def_acquired_arm = 0;
    let mut def_acquired_foot = 0;

    // -- ATK accessories statistic --
    // [0]: Has only 1 accessory
    // [1]: Has only 2 accessories
    // [2]: Has all 3 accessories
    let mut atk_acc_amount_acquired = vec![0; 3];

    // [0]: Has (eyes + weapon)
    // [1]: Has (eyes + sidearms)
    let mut atk_acc_acquired_2 = vec![0; 2];

    // -- One Special Tile statistic --
    let mut has_one_special_tile_amount = 0;
    // [0]: Has only 1 accessory
    // [1]: Has only 2 accessories
    // [2]: Has all 3 accessories
    let mut mono_spc_acc_amount_acquired = vec![0; 3];

    // Has (floating + ground)
    let mut mono_spc_acqired_2 = 0;

    let mut mono_spc_dmg_above_threshold_amount = 0;

    let mut mono_spc_acquired_floating = 0;
    let mut mono_spc_acquired_ground = 0;
    let mut mono_spc_acquired_bg_effect = 0;

    // -- Two Special Tile statistic --
    let mut has_two_special_tile_amount = 0;
    // [0]: Has only 1 accessory
    // [1]: Has only 2 accessories
    // [2]: Has only 3 accessories
    // [3]: Has all 4 accessories
    let mut dual_spc_acc_amount_acquired = vec![0; 4];

    let mut dual_spc_same_color_acquired = 0;
    let mut dual_spc_diff_color_acquired = 0;

    // First dimension [0] is same color, [1] is diff color
    // Second dimension is the acceesories combination:
    // -- Ground effect is 100% guaranteed to be acquired
    // [0]: Has ([ground effect] + floating item + ground item)
    // [1]: Has ([ground effect] + floating item + bg effect)
    // [2]: Has ([ground effect] + ground item + bg effect)
    let mut dual_spc_acquired_3 = vec![vec![0; 3]; 2];

    // First dimension [0] is same color, [1] is diff color
    // Second dimension is the acceesories combination:
    // -- Ground effect is 100% guaranteed to be acquired
    // [0]: Has ([ground effect] + floating item)
    // [1]: Has ([ground effect] + ground item)
    // [2]: Has ([ground effect] + bg effect)
    let mut dual_spc_acquired_2 = vec![vec![0; 3]; 2];

    let mut each_acc_acquired_amount = vec![0; AccPart::COUNT];
    let mut each_char_rarity_count = vec![0; MAX_RARITY_LV as usize];

    let mut chosen_one = 0;

    let config = GameplayConfigManager::new();

    // -- RUN SIMULATE --
    for _ in 0..simulation_count {
        // Create new character
        let char_attr_test = Attribute::roll_attribute(tier_lv, &config);
        let char_module_test = if tier_lv == EVEN_CHANCE_TIER_LV {
            AccessoryModule::_debug_evenly_roll_accessory()
        } else {
            AccessoryModule::roll_accessory(&char_attr_test, &config)
        };
        let char_rarity = Attribute::get_char_rarity(&char_attr_test, &config);

        each_char_rarity_count[char_rarity as usize - 1] += 1;

        for (i, val) in char_module_test
            .accessory_list
            .clone()
            .into_iter()
            .enumerate()
        {
            if val != 0 {
                each_acc_acquired_amount[i] += 1;
            }
        }

        // HP accessories
        let hp_acquired_mapping = vec![
            (char_module_test.accessory_list[AccPart::Head as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::Face as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::Neck as usize] != 0) as usize,
        ];

        hp_acc_amount_acquired[hp_acquired_mapping.iter().sum::<usize>() - 1] += 1;

        if hp_acquired_mapping[0] + hp_acquired_mapping[1] == 2 && hp_acquired_mapping[2] == 0 {
            hp_acc_acquired_2[0] += 1;
        } else if hp_acquired_mapping[0] + hp_acquired_mapping[2] == 2
            && hp_acquired_mapping[1] == 0
        {
            hp_acc_acquired_2[1] += 1;
        } else if hp_acquired_mapping[1] + hp_acquired_mapping[2] == 2
            && hp_acquired_mapping[0] == 0
        {
            hp_acc_acquired_2[2] += 1;
        }

        for (i, val) in hp_acquired_mapping.iter().enumerate() {
            if *val != 0 && hp_acquired_mapping.iter().sum::<usize>() == 1 {
                hp_acc_acquired_1[i] += 1;
            }
        }

        // DEF accessories
        let def_acquired_mapping = vec![
            (char_module_test.accessory_list[AccPart::Body as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::Waist as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::Arm as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::Foot as usize] != 0) as usize,
        ];

        def_acc_amount_acquired[def_acquired_mapping.iter().sum::<usize>() - 1] += 1;

        if def_acquired_mapping.iter().sum::<usize>() == 1 {
            if def_acquired_mapping[0] != 0 {
                def_acc_body_part_acquired_1[0] += 1;
            } else if def_acquired_mapping[1] != 0 {
                def_acc_body_part_acquired_1[1] += 1;
            }
        }

        if def_acquired_mapping.iter().sum::<usize>() == 2 {
            if def_acquired_mapping[0] + def_acquired_mapping[1] == 2 {
                def_acc_body_part_acquired_2[0] += 1;
            } else if def_acquired_mapping[0] + def_acquired_mapping[2] == 2 {
                def_acc_body_part_acquired_2[1] += 1;
            } else if def_acquired_mapping[0] + def_acquired_mapping[3] == 2 {
                def_acc_body_part_acquired_2[2] += 1;
            } else if def_acquired_mapping[1] + def_acquired_mapping[2] == 2 {
                def_acc_body_part_acquired_2[3] += 1;
            } else if def_acquired_mapping[1] + def_acquired_mapping[3] == 2 {
                def_acc_body_part_acquired_2[4] += 1;
            }
        }

        if def_acquired_mapping.iter().sum::<usize>() == 3 {
            if def_acquired_mapping[0] + def_acquired_mapping[1] + def_acquired_mapping[2] == 3 {
                def_acc_body_part_acquired_3[0] += 1;
            } else if def_acquired_mapping[0] + def_acquired_mapping[1] + def_acquired_mapping[3]
                == 3
            {
                def_acc_body_part_acquired_3[1] += 1;
            } else if def_acquired_mapping[0] + def_acquired_mapping[2] + def_acquired_mapping[3]
                == 3
            {
                def_acc_body_part_acquired_3[2] += 1;
            } else if def_acquired_mapping[1] + def_acquired_mapping[2] + def_acquired_mapping[3]
                == 3
            {
                def_acc_body_part_acquired_3[3] += 1;
            }
        }

        if def_acquired_mapping[2] != 0 {
            def_acquired_arm += 1;
        }

        if def_acquired_mapping[3] != 0 {
            def_acquired_foot += 1;
        }

        // ATK accessories
        let atk_acquired_mapping = vec![
            (char_module_test.accessory_list[AccPart::Eyes as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::Weapon as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::Sidearms as usize] != 0) as usize,
        ];

        atk_acc_amount_acquired[atk_acquired_mapping.iter().sum::<usize>() - 1] += 1;

        if atk_acquired_mapping.iter().sum::<usize>() == 2 {
            if atk_acquired_mapping[1] != 0 {
                atk_acc_acquired_2[0] += 1;
            } else if atk_acquired_mapping[2] != 0 {
                atk_acc_acquired_2[1] += 1;
            }
        }

        // One Special Tile
        let mono_spc_acquired_mapping = vec![
            (char_module_test.accessory_list[AccPart::FloatingItem1 as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::GroundItem1 as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::BackgroundEffect1 as usize] != 0) as usize,
        ];

        // low tier lv characters may not have special tile
        let special_tile = char_attr_test.get_special_tile();
        if mono_spc_acquired_mapping.iter().sum::<usize>() > 0 {
            mono_spc_acc_amount_acquired[mono_spc_acquired_mapping.iter().sum::<usize>() - 1] += 1;

            if special_tile.elem1_boost_val > MONO_SPC_PREM_THRESHOLD {
                mono_spc_dmg_above_threshold_amount += 1;
            }

            if mono_spc_acquired_mapping[0] != 0 {
                mono_spc_acquired_floating += 1;
            }
            if mono_spc_acquired_mapping[1] != 0 {
                mono_spc_acquired_ground += 1;
            }
            if mono_spc_acquired_mapping[2] != 0 {
                mono_spc_acquired_bg_effect += 1;
            }

            if mono_spc_acquired_mapping.iter().sum::<usize>() == 2 {
                mono_spc_acqired_2 += 1;
            }

            has_one_special_tile_amount += 1;
        }

        // Two Special Tile
        let dual_spc_acquired_mapping = vec![
            (char_module_test.accessory_list[AccPart::GroundEffect as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::FloatingItem2 as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::GroundItem2 as usize] != 0) as usize,
            (char_module_test.accessory_list[AccPart::BackgroundEffect2 as usize] != 0) as usize,
        ];

        let is_same_color = (special_tile.element1 != Element::Unknown
            && special_tile.element2 != Element::Unknown)
            && (special_tile.element1 == special_tile.element2);

        // low tier lv characters may not have special tile
        if dual_spc_acquired_mapping.iter().sum::<usize>() > 0 {
            has_two_special_tile_amount += 1;
            dual_spc_acc_amount_acquired[dual_spc_acquired_mapping.iter().sum::<usize>() - 1] += 1;

            if is_same_color {
                dual_spc_same_color_acquired += 1;
            } else {
                dual_spc_diff_color_acquired += 1;
            }

            if dual_spc_acquired_mapping.iter().sum::<usize>() == 3 {
                // dual_spc_acquired_mapping[0](ground effect) is guaranteed 100% to be acquired
                if dual_spc_acquired_mapping[1] + dual_spc_acquired_mapping[2] == 2 {
                    dual_spc_acquired_3[is_same_color as usize][0] += 1;
                } else if dual_spc_acquired_mapping[1] + dual_spc_acquired_mapping[3] == 2 {
                    dual_spc_acquired_3[is_same_color as usize][1] += 1;
                } else if dual_spc_acquired_mapping[2] + dual_spc_acquired_mapping[3] == 2 {
                    dual_spc_acquired_3[is_same_color as usize][2] += 1;
                }
            }

            if dual_spc_acquired_mapping.iter().sum::<usize>() == 2 {
                // dual_spc_acquired_mapping[0](ground effect) is guaranteed 100% to be acquired
                if dual_spc_acquired_mapping[1] == 1 {
                    dual_spc_acquired_2[is_same_color as usize][0] += 1;
                } else if dual_spc_acquired_mapping[2] == 1 {
                    dual_spc_acquired_2[is_same_color as usize][1] += 1;
                } else if dual_spc_acquired_mapping[3] == 1 {
                    dual_spc_acquired_2[is_same_color as usize][2] += 1;
                }
            }
        }

        // find the chosen one
        let mut temp_list = char_module_test.accessory_list.clone();
        temp_list[AccPart::BackgroundEffect1 as usize] = std::cmp::max(
            temp_list[AccPart::BackgroundEffect1 as usize],
            temp_list[AccPart::BackgroundEffect2 as usize],
        );
        temp_list.pop();

        if !temp_list.iter().any(|x| *x == 0) {
            chosen_one += 1;
        }
    }

    // HP accessories statistic log
    result_log += &"- HP accessory amount -\n".to_owned();

    let mut sum = 0.0;
    for (i, val) in hp_acc_amount_acquired.into_iter().enumerate().rev() {
        let percentage = val as f64 * 100.0 / simulation_count as f64;
        sum += percentage;

        result_log += &format!(" Has {} accessory(s): {}%\n", i + 1, percentage).to_owned();
    }
    result_log += &format!(" ###### Total: {}%\n", sum.to_owned());

    sum = 0.0;
    for (i, val) in hp_acc_acquired_2.into_iter().enumerate() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" {}: {}%\n", HP_MULTI_PART_LOG[i], percentage).to_owned();
    }
    result_log += &format!(" ###### Total: {}%\n", sum.to_owned());

    sum = 0.0;
    for (i, val) in hp_acc_acquired_1.into_iter().enumerate() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" {}: {}%\n", HP_SINGLE_PART_LOG[i], percentage).to_owned();
    }
    result_log += &format!(" ###### Total: {}%\n", sum.to_owned());

    // DEF accessory log
    result_log += &"\n- DEF accessory amount -\n".to_owned();
    sum = 0.0;
    for (i, val) in def_acc_amount_acquired.into_iter().enumerate().rev() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" Has {} accessory(s): {}%\n", i + 1, percentage).to_owned();
    }
    result_log += &format!(" ###### Total: {}%\n", sum.to_owned());

    result_log += &format!(
        " {}: {}%\n",
        DEF_2_BODY_PART_LOG[0],
        to_percent(def_acc_body_part_acquired_2[0], simulation_count)
    )
    .to_owned();
    result_log += &format!(" ---\n");

    sum = 0.0;
    for (i, val) in def_acc_body_part_acquired_1.into_iter().enumerate() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" {}: {}%\n", DEF_1_BODY_PART_LOG[i], percentage).to_owned();
    }
    result_log += &format!(" ###### Has only 1 body part Total: {}%\n", sum.to_owned());

    result_log += &format!(
        " Has arm: {}%\n",
        to_percent(def_acquired_arm, simulation_count)
    );
    result_log += &format!(
        " Has foot: {}%\n",
        to_percent(def_acquired_foot, simulation_count)
    );
    result_log += &format!(" ---\n");

    sum = 0.0;
    for (i, val) in def_acc_body_part_acquired_2.into_iter().enumerate() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" {}: {}%\n", DEF_2_BODY_PART_LOG[i], percentage);
    }
    result_log += &format!(" ###### Has only 2 body part Total: {}%\n", sum.to_owned());

    sum = 0.0;
    for (i, val) in def_acc_body_part_acquired_3.into_iter().enumerate() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" {}: {}%\n", DEF_3_BODY_PART_LOG[i], percentage);
    }
    result_log += &format!(" ###### Has only 3 body part Total: {}%\n", sum.to_owned());

    // ATK accessories
    result_log += &"\n- ATK accessory amount -\n".to_owned();
    sum = 0.0;
    for (i, val) in atk_acc_amount_acquired.into_iter().enumerate().rev() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" Has {} accessory(s): {}%\n", i + 1, percentage).to_owned();
    }
    result_log += &format!(" ###### Total: {}%\n", sum.to_owned());

    sum = 0.0;
    for (i, val) in atk_acc_acquired_2.into_iter().enumerate() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" {}: {}%\n", ATK_2_PART_LOG[i], percentage);
    }
    result_log += &format!(" ###### Has only 2 body part Total: {}%\n", sum.to_owned());

    // One Special Tile accessories
    result_log += &"\n- MONO SPC accessory amount -\n".to_owned();
    result_log += &format!(
        " Has at least one special tiles: {}%\n",
        to_percent(has_one_special_tile_amount, simulation_count)
    );
    result_log += &format!(" ---\n");

    sum = 0.0;
    for (i, val) in mono_spc_acc_amount_acquired.into_iter().enumerate().rev() {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" Has {} accessory(s): {}%\n", i + 1, percentage).to_owned();
    }
    result_log += &format!(" ###### Total: {}%\n", sum.to_owned());

    result_log += &format!(
        " {}: {}%\n",
        MONO_SPC_2_PART_LOG,
        to_percent(mono_spc_acqired_2, simulation_count)
    );

    result_log += &format!(
        " Has floating: {}%\n",
        to_percent(mono_spc_acquired_floating, simulation_count)
    );
    result_log += &format!(
        " Has ground: {}%\n",
        to_percent(mono_spc_acquired_ground, simulation_count)
    );
    result_log += &format!(
        " Has bg effect: {}%\n",
        to_percent(mono_spc_acquired_bg_effect, simulation_count)
    );
    result_log += &format!(" ---\n");

    result_log += &format!(
        " ###### Special tile damage above threshold: {}%\n",
        mono_spc_dmg_above_threshold_amount as f64 * 100.0 / simulation_count as f64
    );

    // Second Special Tile accessory
    result_log += &"\n- DUAL SPC accessory amount -\n".to_owned();
    result_log += &format!(
        " Has two special tiles: {}%\n",
        to_percent(has_two_special_tile_amount, simulation_count)
    );
    result_log += &format!(
        " Has same color: {}% ({}% in 2 spc possessors)\n",
        to_percent(dual_spc_same_color_acquired, simulation_count),
        to_percent(dual_spc_same_color_acquired, has_two_special_tile_amount)
    );
    result_log += &format!(
        " Has diff color: {}% ({}% in 2 spc possessors)\n",
        to_percent(dual_spc_diff_color_acquired, simulation_count),
        to_percent(dual_spc_diff_color_acquired, has_two_special_tile_amount)
    );
    result_log += &format!(" ---\n");

    sum = 0.0;
    for (i, val) in dual_spc_acc_amount_acquired
        .clone()
        .into_iter()
        .enumerate()
        .rev()
    {
        let percentage = to_percent(val, simulation_count);
        sum += percentage;

        result_log += &format!(" Has {} accessory(s): {}%\n", i + 1, percentage).to_owned();
    }
    result_log += &format!(" ###### Total: {}%\n", sum.to_owned());

    result_log += &format!(
        "\n All 4 accessories acquired cases in {} times rolled:\n",
        simulation_count
    );
    result_log += &format!(
        " {}: {}\n",
        DUAL_SPC_4_PART_LOG, dual_spc_acc_amount_acquired[3]
    );

    result_log += &format!(
        "\n Only 3 accessories acquired cases in {} times rolled:\n",
        simulation_count
    );
    result_log += &format!(" -- If NFT has same elements --\n");
    sum = 0.0;
    for (i, val) in dual_spc_acquired_3[0].clone().into_iter().enumerate() {
        sum += val as f64;

        result_log += &format!(" {}: {}\n", DUAL_SPC_3_PART_LOG[i], val);
    }
    result_log += &format!(" -- If NFT has diff elements --\n");
    for (i, val) in dual_spc_acquired_3[1].clone().into_iter().enumerate() {
        sum += val as f64;

        result_log += &format!(" {}: {}\n", DUAL_SPC_3_PART_LOG[i], val);
    }
    result_log += &format!(" ###### Has only 3 part Total: {}\n", sum.to_owned());

    result_log += &format!(
        "\n Only 2 accessories acquired cases in {} times rolled:\n",
        simulation_count
    );
    result_log += &format!(" -- If NFT has same elements --\n");
    sum = 0.0;
    for (i, val) in dual_spc_acquired_2[0].clone().into_iter().enumerate() {
        sum += val as f64;

        result_log += &format!(" {}: {}\n", DUAL_SPC_2_PART_LOG[i], val);
    }
    result_log += &format!(" -- If NFT has diff elements --\n");
    for (i, val) in dual_spc_acquired_2[1].clone().into_iter().enumerate() {
        sum += val as f64;

        result_log += &format!(" {}: {}\n", DUAL_SPC_2_PART_LOG[i], val);
    }
    result_log += &format!(" ###### Has only 2 part Total: {}\n", sum.to_owned());

    result_log += &format!(
        "\n--- Summary in Tier: [{}], and [{}] times rolled ---\n Each character rarity:\n",
        tier_lv, simulation_count
    );

    for (i, val) in each_char_rarity_count.into_iter().enumerate() {
        result_log += &format!(
            " Rarity {}: {} ({}%)\n",
            i + 1,
            val,
            to_percent(val, simulation_count)
        );
    }
    result_log += &format!("\n Each accessory acquired:\n");

    for (i, val) in each_acc_acquired_amount.into_iter().enumerate() {
        result_log += &format!(
            " {:20}: {} ({}%)\n",
            ACCESSORY_EACH_PART_LOG[i],
            val,
            to_percent(val, simulation_count)
        );
    }

    result_log += &format!(
        "\n The chosen one who owned all accessories: {}\n",
        chosen_one
    );

    log::debug!("{}", result_log);
    result_log
}

fn to_percent(val: u32, simulation_count: u32) -> f64 {
    val as f64 * 100.0 / simulation_count as f64
}
