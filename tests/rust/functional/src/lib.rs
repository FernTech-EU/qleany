// Functional tests for the generated Rust project.
// Each test module mirrors the equivalent C++/Qt functional test suite.

#[cfg(test)]
mod helpers;

#[cfg(test)]
mod test_root_controller;

#[cfg(test)]
mod test_tag_controller;

#[cfg(test)]
mod test_task_controller;

#[cfg(test)]
mod test_project_list_fields;

#[cfg(test)]
mod test_undo_redo;

#[cfg(test)]
mod test_long_operation;

#[cfg(test)]
mod test_project_controller;

#[cfg(test)]
mod test_system_controller;

#[cfg(test)]
mod test_team_member_controller;

#[cfg(test)]
mod test_category_controller;

#[cfg(test)]
mod test_workspace_controller;

#[cfg(test)]
mod test_project_settings_controller;

#[cfg(test)]
mod test_comment_controller;

#[cfg(test)]
mod test_feature_use_cases;
