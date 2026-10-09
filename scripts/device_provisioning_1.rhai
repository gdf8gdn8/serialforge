// ============================================================================
// Rhai Asynchronous Device Provisioning Policy Engine
// ============================================================================

const PROVISION_TIMEOUT_SEC = 30;
const MAX_RETRY_COUNT = 3;
const PROTOCOL_MQTT = "mqtt";
const PROTOCOL_HTTPS = "https";

fn get_device_profile(device_model) {
    if device_model == "sensor_node_v1" {
        return #{
            model: device_model,
            protocol: global::PROTOCOL_MQTT,
            port: 8883,
            firmware_version: "1.4.2",
            requires_auth: true,
            heartbeat_interval: 60
        };
    } else if device_model == "gateway_edge_v2" {
        return #{
            model: device_model,
            protocol: global::PROTOCOL_HTTPS,
            port: 443,
            firmware_version: "2.1.0",
            requires_auth: true,
            heartbeat_interval: 15
        };
    }
    return #{
        model: "generic_device",
        protocol: global::PROTOCOL_MQTT,
        port: 1883,
        firmware_version: "1.0.0",
        requires_auth: false,
        heartbeat_interval: 120
    };
}

fn validate_device_config(config) {
    if !config.contains("device_id") {
        print("[ERROR] Config missing 'device_id'");
        return false;
    }
    if !config.contains("mac_address") {
        print("[ERROR] Config missing 'mac_address'");
        return false;
    }
    if config.mac_address.len() < 12 {
        print("[ERROR] Invalid MAC address format");
        return false;
    }
    return true;
}

fn generate_provisioning_payload(device_id, profile) {
    return #{
        target_id: device_id,
        assigned_protocol: profile.protocol,
        server_port: profile.port,
        target_fw: profile.firmware_version,
        timeout: global::PROVISION_TIMEOUT_SEC,
        max_retries: global::MAX_RETRY_COUNT,
        status: "ready_to_provision"
    };
}

fn evaluate_provisioning_response(status_code, response_body) {
    if status_code == 200 {
        print("[SUCCESS] Device provisioned successfully. Response: " + response_body);
        return #{ success: true, action: "activate" };
    } else if status_code == 401 || status_code == 403 {
        print("[WARN] Authentication failed during provisioning. Status: " + status_code);
        return #{ success: false, action: "retry_auth" };
    } else {
        print("[FATAL] Provisioning failed with status: " + status_code);
        return #{ success: false, action: "abort" };
    }
}
