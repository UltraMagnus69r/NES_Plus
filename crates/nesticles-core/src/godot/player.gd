extends CharacterBody2D

# Fixed Nesticles runtime template.
# Movement numbers come from res://data/level.json. This script does not read
# asset names, and it is not the Rust SMB1 integrator.

var move_speed := 96.0
var jump_velocity := -280.0
var gravity := 900.0
var test_axis := 0.0
var test_jump := false

func _ready() -> void:
	_load_motion()
	_ensure_input()
	floor_snap_length = 8.0
	var camera := get_node_or_null("Camera") as Camera2D
	if camera != null:
		camera.make_current()

func _physics_process(delta: float) -> void:
	var axis := test_axis
	if axis == 0.0:
		axis = Input.get_axis("move_left", "move_right")
	if not is_on_floor():
		velocity.y += gravity * delta
	var jump := test_jump or Input.is_action_just_pressed("jump")
	test_jump = false
	if jump and is_on_floor():
		velocity.y = jump_velocity
	velocity.x = axis * move_speed
	move_and_slide()

func _load_motion() -> void:
	var path := "res://data/level.json"
	if not FileAccess.file_exists(path):
		return
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	if typeof(parsed) != TYPE_DICTIONARY:
		return
	var data: Dictionary = parsed
	move_speed = float(data.get("move_speed", move_speed))
	jump_velocity = float(data.get("jump_velocity", jump_velocity))
	gravity = float(data.get("gravity", gravity))

func _ensure_input() -> void:
	_bind("move_left", [KEY_A, KEY_LEFT])
	_bind("move_right", [KEY_D, KEY_RIGHT])
	_bind("jump", [KEY_SPACE, KEY_W, KEY_UP])

func _bind(action: String, keys: Array) -> void:
	if not InputMap.has_action(action):
		InputMap.add_action(action)
	for key in keys:
		var event := InputEventKey.new()
		event.physical_keycode = key
		if not InputMap.action_has_event(action, event):
			InputMap.action_add_event(action, event)
