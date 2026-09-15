use serde_json::json;

pub fn json_validation_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "schema": {
                "type": "object",
                "properties": {
                    "version": {
                        "type": "integer",
                        "minimum": 6,
                        "maximum": 6
                    }
                },
                "required": ["version"],
                "additionalProperties": false
            },
            "global": {
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["cpp-qt", "rust"]
                    },
                    "application_name": {
                        "type": "string"
                    },
                    "organisation": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string"
                            },
                            "domain": {
                                "type": "string"
                            }
                        },
                        "required": ["name", "domain"],
                        "additionalProperties": false
                    },
                    "prefix_path": {
                        "type": "string"
                    }
                },
                "required": ["language", "application_name", "organisation", "prefix_path"],
                "additionalProperties": false
            },
            "entities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string"
                        },
                        "inherits_from": {
                            "type": "string"
                        },
                        "single_model": {
                            "type": "boolean"
                        },
                        "only_for_heritage": {
                            "type": "boolean"
                        },
                        "undoable": {
                            "type": "boolean"
                        },
                        "fields": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {
                                        "type": "string"
                                    },
                                    "type": {
                                        "type": "string"
                                    },
                                    "entity": {
                                        "type": "string"
                                    },
                                    "relationship": {
                                        "type": "string"
                                    },
                                    "strong": {
                                        "type": "boolean"
                                    },
                                    "list_model": {
                                        "type": "boolean"
                                    },
                                    "optional": {
                                        "type": "boolean"
                                    },
                                    "is_list": {
                                        "type": "boolean"
                                    },
                                    "list_model_displayed_field": {
                                        "type": "string"
                                    },
                                    "enum_name": {
                                        "type": "string"
                                    },
                                    "enum_values": {
                                        "type": "array",
                                        "items": {
                                            "type": "string"
                                        }
                                    },
                                },
                                "required": ["name", "type"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["name", "fields"],
                    "additionalProperties": false
                }
            },
            "features": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string"
                        },
                        "use_cases": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {
                                        "type": "string"
                                    },
                                    "entities": {
                                        "type": "array",
                                        "items": {
                                            "type": "string"
                                        }
                                    },
                                    "undoable": {
                                        "type": "boolean"
                                    },
                                    "read_only": {
                                        "type": "boolean"
                                    },
                                    "long_operation": {
                                        "type": "boolean"
                                    },
                                    "dto_in": {
                                        "type": "object",
                                        "properties": {
                                            "name": {
                                                "type": "string"
                                            },
                                            "fields": {
                                                "type": "array",
                                                "items": {
                                                    "type": "object",
                                                    "properties": {
                                                        "name": {
                                                            "type": "string"
                                                        },
                                                        "type": {
                                                            "type": "string"
                                                        },
                                                        "optional": {
                                                            "type": "boolean"
                                                        },
                                                        "is_list": {
                                                            "type": "boolean"
                                                        },
                                                        "enum_name": {
                                                            "type": "string"
                                                        },
                                                        "enum_values": {
                                                            "type": "array",
                                                            "items": {
                                                                "type": "string"
                                                            }
                                                        },
                                                    },
                                                    "required": ["name", "type"],
                                                    "additionalProperties": false
                                                }
                                            }
                                        },
                                        "required": ["name", "fields"],
                                        "additionalProperties": false
                                    },
                                    "dto_out": {
                                        "type": "object",
                                        "properties": {
                                            "name": {
                                                "type": "string"
                                            },
                                            "fields": {
                                                "type": "array",
                                                "items": {
                                                    "type": "object",
                                                    "properties": {
                                                        "name": {
                                                            "type": "string"
                                                        },
                                                        "type": {
                                                            "type": "string"
                                                        },
                                                        "optional": {
                                                            "type": "boolean"
                                                        },
                                                        "is_list": {
                                                            "type": "boolean"
                                                        },
                                                        "enum_name": {
                                                            "type": "string"
                                                        },
                                                        "enum_values": {
                                                            "type": "array",
                                                            "items": {
                                                                "type": "string"
                                                            }
                                                        },
                                                    },
                                                    "required": ["name", "type"],
                                                    "additionalProperties": false
                                                }
                                            }
                                        },
                                        "required": ["name", "fields"],
                                        "additionalProperties": false
                                    }
                                },
                                "required": ["name"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["name", "use_cases"],
                    "additionalProperties": false
                }
            },
            "ui": {
                "type": "object",
                "properties": {
                    "rust_cli": {
                        "type": "boolean"
                    },
                    "rust_slint": {
                        "type": "boolean"
                    },
                    "rust_teksilo": {
                        "type": "boolean"
                    },
                    "cpp_qt_qtwidgets": {
                        "type": "boolean"
                    },
                    "cpp_qt_qtquick": {
                        "type": "boolean"
                    },
                    "rust_ios": {
                        "type": "boolean"
                    },
                    "rust_android": {
                        "type": "boolean"
                    }

                },
                "additionalProperties": false
            }
        },
        "required": ["schema", "global", "entities", "features", "ui"],
        "additionalProperties": false
    })
}
