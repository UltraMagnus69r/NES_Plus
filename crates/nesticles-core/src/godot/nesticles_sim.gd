extends RefCounted

# Nesticles SMB1-style step for the 1080p play field.
# One call is one 60 Hz tick. Positions are i32 with 4096 units per screen pixel.
# Numbers come from the JSON written by Rust. This file does not read asset names.
#
# Supported here: walk, run, skid, jump, hold gravity, Peach hover, max fall,
# solid collision, corner nudge, goomba wake/walk/wall reverse, stomp, side defeat,
# and goal flags.
# Not in this build: pipes, vines, springs, hazards, items, power-ups,
# question blocks, and every enemy that is not a goomba.

const I32_MAX := 2147483647
const I32_MIN := -2147483648

var fp := 4096
var tile_fp := 294912
var hit_w := 221184
var hit_h := 294912
var screen_w_fp := 7864320
var screen_h_fp := 4423680
var corner_fp := 55296
var nudge_fp := 18432
var view_margin := 294912

var max_walk := 0
var max_run := 0
var walk_accel := 0
var run_accel := 0
var release_decel := 0
var skid_decel := 0
var jump_slow := 0
var jump_fast := 0
var jump_fast_at := 0
var gravity_hold := 0
var gravity_normal := 0
var max_fall := 0
var hover := false
var stomp_vy := 0
var stomp_vy_held := 0
var walker_vx := 0
var walker_gravity := 0
var walker_max_fall := 0

var body_x := 0
var body_y := 0
var body_w := 0
var body_h := 0
var body_vx := 0
var body_vy := 0
var facing := 1
var on_ground := false
var is_jumping := false
var run_timer := 0
var hover_timer := 0
var lives := 3
var spawn_x := 0
var spawn_y := 0
var level_w := 0
var level_h := 0
var camera_x := 0
var camera_y := 0
var won := false
var solids: Array = []
var enemies: Array = []
var goals: Array = []
var last_events: Dictionary = {}

func load_file(path: String) -> String:
	if not FileAccess.file_exists(path):
		return "level data is missing"
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if typeof(parsed) != TYPE_DICTIONARY:
		return "level data is not a dictionary"
	var data: Dictionary = parsed
	if not data.has("sim"):
		return "level data has no sim block"
	return load_state(data["sim"])

func load_state(state: Dictionary) -> String:
	if typeof(state) != TYPE_DICTIONARY:
		return "sim state is not a dictionary"
	var space: Dictionary = state["space"]
	var motion: Dictionary = state["motion"]
	fp = int(space["fp"])
	tile_fp = int(space["tile_fp"])
	hit_w = int(space["hit_w"])
	hit_h = int(space["hit_h"])
	screen_w_fp = int(space["screen_w_fp"])
	screen_h_fp = int(space["screen_h_fp"])
	corner_fp = int(space["corner_fp"])
	nudge_fp = int(space["nudge_fp"])
	view_margin = int(space["view_margin"])
	max_walk = int(motion["max_walk"])
	max_run = int(motion["max_run"])
	walk_accel = int(motion["walk_accel"])
	run_accel = int(motion["run_accel"])
	release_decel = int(motion["release_decel"])
	skid_decel = int(motion["skid_decel"])
	jump_slow = int(motion["jump_slow"])
	jump_fast = int(motion["jump_fast"])
	jump_fast_at = int(motion["jump_fast_at"])
	gravity_hold = int(motion["gravity_hold"])
	gravity_normal = int(motion["gravity_normal"])
	max_fall = int(motion["max_fall"])
	hover = bool(motion["hover"])
	stomp_vy = int(motion["stomp_vy"])
	stomp_vy_held = int(motion["stomp_vy_held"])
	walker_vx = int(motion["walker_vx"])
	walker_gravity = int(motion["walker_gravity"])
	walker_max_fall = int(motion["walker_max_fall"])
	spawn_x = int(state["spawn_x"])
	spawn_y = int(state["spawn_y"])
	level_w = int(state["level_w"])
	level_h = int(state["level_h"])
	won = bool(state.get("won", false))
	camera_x = 0
	camera_y = 0
	_read_body(state["player"])
	solids = []
	for raw in state["solids"]:
		solids.append(_solid(raw))
	enemies = []
	for raw in state.get("enemies", []):
		if str(raw["kind"]) != "goomba":
			return "unsupported enemy %s" % str(raw["kind"])
		enemies.append(_enemy(raw))
	goals = []
	for raw in state.get("goals", []):
		goals.append(_rect(raw))
	last_events = {"jump": false, "stomp": false, "hurt": false, "goal": false}
	return ""

func step(pad: Dictionary) -> Dictionary:
	var events := {"jump": false, "stomp": false, "hurt": false, "goal": false}
	if won:
		last_events = events
		return events
	_update_camera()
	var prev_y := body_y
	var prev_vy := body_vy
	if _step_player(pad):
		events["jump"] = true
	_step_enemies()
	var contact := _contacts(pad, prev_y, prev_vy)
	if bool(contact["stomp"]):
		events["stomp"] = true
	if bool(contact["hurt"]):
		events["hurt"] = true
	if _goals():
		events["goal"] = true
	if body_y > screen_h_fp:
		_defeat()
		events["hurt"] = true
	last_events = events
	return events

func difference(sample: Dictionary) -> String:
	var player: Dictionary = sample["player"]
	var err := _cmp_int("player.x", body_x, player["x"])
	if err != "":
		return err
	err = _cmp_int("player.y", body_y, player["y"])
	if err != "":
		return err
	err = _cmp_int("player.vx", body_vx, player["vx"])
	if err != "":
		return err
	err = _cmp_int("player.vy", body_vy, player["vy"])
	if err != "":
		return err
	err = _cmp_int("player.w", body_w, player["w"])
	if err != "":
		return err
	err = _cmp_int("player.h", body_h, player["h"])
	if err != "":
		return err
	err = _cmp_int("player.facing", facing, player["facing"])
	if err != "":
		return err
	err = _cmp_bool("player.on_ground", on_ground, player["on_ground"])
	if err != "":
		return err
	err = _cmp_bool("player.is_jumping", is_jumping, player["is_jumping"])
	if err != "":
		return err
	err = _cmp_int("player.run_timer", run_timer, player["run_timer"])
	if err != "":
		return err
	err = _cmp_int("player.hover_timer", hover_timer, player["hover_timer"])
	if err != "":
		return err
	err = _cmp_int("player.lives", lives, player["lives"])
	if err != "":
		return err
	err = _cmp_bool("won", won, sample["won"])
	if err != "":
		return err
	err = _cmp_bool("jump", bool(last_events["jump"]), sample["jump"])
	if err != "":
		return err
	err = _cmp_bool("stomp", bool(last_events["stomp"]), sample["stomp"])
	if err != "":
		return err
	err = _cmp_bool("hurt", bool(last_events["hurt"]), sample["hurt"])
	if err != "":
		return err
	err = _cmp_bool("goal", bool(last_events["goal"]), sample["goal"])
	if err != "":
		return err
	var expected: Array = sample["enemies"]
	if enemies.size() != expected.size():
		return "enemy count %s != %s" % [enemies.size(), expected.size()]
	for i in enemies.size():
		var actual: Dictionary = enemies[i]
		var want: Dictionary = expected[i]
		err = _cmp_int("enemy.x", int(actual["x"]), want["x"])
		if err != "":
			return err
		err = _cmp_int("enemy.y", int(actual["y"]), want["y"])
		if err != "":
			return err
		err = _cmp_int("enemy.vx", int(actual["vx"]), want["vx"])
		if err != "":
			return err
		err = _cmp_int("enemy.vy", int(actual["vy"]), want["vy"])
		if err != "":
			return err
		err = _cmp_int("enemy.facing", int(actual["facing"]), want["facing"])
		if err != "":
			return err
		err = _cmp_bool("enemy.alive", bool(actual["alive"]), want["alive"])
		if err != "":
			return err
		err = _cmp_bool("enemy.awake", bool(actual["awake"]), want["awake"])
		if err != "":
			return err
		err = _cmp_bool("enemy.on_ground", bool(actual["on_ground"]), want["on_ground"])
		if err != "":
			return err
	return ""

func _read_body(raw: Dictionary) -> void:
	body_x = int(raw["x"])
	body_y = int(raw["y"])
	body_w = int(raw["w"])
	body_h = int(raw["h"])
	body_vx = int(raw["vx"])
	body_vy = int(raw["vy"])
	facing = int(raw["facing"])
	on_ground = bool(raw["on_ground"])
	is_jumping = bool(raw["is_jumping"])
	run_timer = int(raw["run_timer"])
	hover_timer = int(raw["hover_timer"])
	lives = int(raw["lives"])

func _solid(raw: Dictionary) -> Dictionary:
	return {
		"x": int(raw["x"]),
		"y": int(raw["y"]),
		"w": int(raw["w"]),
		"h": int(raw["h"]),
		"cell_x": int(raw["cell_x"]),
		"cell_y": int(raw["cell_y"]),
		"broken": bool(raw.get("broken", false)),
	}

func _enemy(raw: Dictionary) -> Dictionary:
	return {
		"x": int(raw["x"]),
		"y": int(raw["y"]),
		"w": int(raw["w"]),
		"h": int(raw["h"]),
		"vx": int(raw["vx"]),
		"vy": int(raw["vy"]),
		"facing": int(raw["facing"]),
		"alive": bool(raw["alive"]),
		"awake": bool(raw["awake"]),
		"on_ground": bool(raw["on_ground"]),
		"kind": "goomba",
	}

func _rect(raw: Dictionary) -> Dictionary:
	return {
		"x": int(raw["x"]),
		"y": int(raw["y"]),
		"w": int(raw["w"]),
		"h": int(raw["h"]),
	}

func _step_player(pad: Dictionary) -> bool:
	var jumped := _step_motion(pad)
	var rect := _player_rect()
	var hit := _move_body(rect, body_vx, body_vy, true)
	body_x = int(rect["x"])
	body_y = int(rect["y"])
	_apply_y(hit)
	return jumped

func _step_motion(pad: Dictionary) -> bool:
	if bool(pad["run_held"]):
		run_timer = 10
	elif run_timer > 0:
		run_timer -= 1
	var running: bool = run_timer > 0
	var max_speed := max_walk
	var accel := walk_accel
	if running:
		max_speed = max_run
		accel = run_accel
	var dir := 0
	if bool(pad["left"]) and not bool(pad["right"]):
		dir = -1
	elif bool(pad["right"]) and not bool(pad["left"]):
		dir = 1
	_step_horizontal(dir, accel, max_speed)
	var jumped := false
	if on_ground:
		if hover:
			hover_timer = 90
		body_vy = 0
		is_jumping = false
		if bool(pad["jump_pressed"]):
			var fast: bool = iabs(body_vx) >= jump_fast_at
			if fast:
				body_vy = jump_fast
			else:
				body_vy = jump_slow
			on_ground = false
			is_jumping = true
			jumped = true
	if not on_ground:
		var hovering: bool = hover and body_vy >= 0 and bool(pad["jump_held"]) and hover_timer > 0
		if hovering:
			var phase := idiv(90 - hover_timer, 4)
			if phase % 2 == 0:
				body_vy = 1024
			else:
				body_vy = -1024
			hover_timer = sat_sub(hover_timer, 1)
		else:
			if hover and not bool(pad["jump_held"]):
				hover_timer = 0
			var hold: bool = body_vy < 0 and bool(pad["jump_held"]) and is_jumping
			var gravity := gravity_normal
			if hold:
				gravity = gravity_hold
			body_vy = imin(sat_add(body_vy, gravity), max_fall)
			if not bool(pad["jump_held"]) or body_vy >= 0:
				is_jumping = false
	if facing == 0:
		facing = 1
	return jumped

func _step_horizontal(dir: int, accel: int, max_speed: int) -> void:
	if on_ground and dir != 0 and body_vx != 0 and signum(body_vx) != dir:
		body_vx = approach_zero(body_vx, skid_decel)
		facing = dir
		return
	if dir != 0:
		var same: bool = body_vx == 0 or signum(body_vx) == dir
		if same:
			if iabs(body_vx) < max_speed:
				body_vx = sat_add(body_vx, sat_mul(dir, accel))
				if signum(body_vx) == dir and iabs(body_vx) > max_speed:
					body_vx = sat_mul(dir, max_speed)
			elif on_ground:
				body_vx = pull_toward(body_vx, max_speed, release_decel)
		else:
			body_vx = sat_add(body_vx, sat_mul(dir, accel))
			if signum(body_vx) == dir and iabs(body_vx) > max_speed:
				body_vx = sat_mul(dir, max_speed)
		if on_ground:
			facing = dir
	elif on_ground:
		if iabs(body_vx) > max_speed:
			body_vx = pull_toward(body_vx, max_speed, release_decel)
		else:
			body_vx = approach_zero(body_vx, release_decel)

func _apply_y(hit: Dictionary) -> void:
	var which := str(hit["y"])
	if which == "floor":
		on_ground = true
		body_vy = 0
		is_jumping = false
		if hover:
			hover_timer = 90
	elif which == "ceiling":
		body_vy = 0
		on_ground = false
		is_jumping = false
	else:
		if body_vy != 0 or not _on_floor(_player_rect()):
			on_ground = false

func _step_enemies() -> void:
	for enemy in enemies:
		if not bool(enemy["alive"]):
			continue
		if not bool(enemy["awake"]):
			if not _sees(int(enemy["x"]), int(enemy["y"]), int(enemy["w"]), int(enemy["h"])):
				continue
			_wake(enemy)
		_walk(enemy)
	_bump_goombas()

func _wake(enemy: Dictionary) -> void:
	enemy["awake"] = true
	var toward := 1
	if body_x < int(enemy["x"]):
		toward = -1
	enemy["facing"] = toward
	enemy["vx"] = sat_mul(toward, walker_vx)

func _walk(enemy: Dictionary) -> void:
	var face := int(enemy["facing"])
	if face == 0:
		face = -1
	face = signum(face)
	enemy["facing"] = face
	enemy["vx"] = sat_mul(face, walker_vx)
	if bool(enemy["on_ground"]):
		enemy["vy"] = 0
	else:
		enemy["vy"] = imin(sat_add(int(enemy["vy"]), walker_gravity), walker_max_fall)
	_integrate(enemy)
	if bool(enemy["on_ground"]) and not _on_floor(enemy):
		enemy["on_ground"] = false

func _integrate(enemy: Dictionary) -> void:
	var rect := {
		"x": int(enemy["x"]),
		"y": int(enemy["y"]),
		"w": int(enemy["w"]),
		"h": int(enemy["h"]),
	}
	var vx := int(enemy["vx"])
	var vy := int(enemy["vy"])
	var hit := _move_body(rect, vx, vy, false)
	enemy["x"] = int(rect["x"])
	enemy["y"] = int(rect["y"])
	if bool(hit["wall"]) and vx != 0:
		var face := -signum(int(enemy["facing"]))
		if face == 0:
			face = -1
		enemy["facing"] = face
		enemy["vx"] = sat_mul(face, iabs(vx))
	var which := str(hit["y"])
	if which == "floor":
		enemy["vy"] = 0
		enemy["on_ground"] = true
	elif which == "ceiling":
		enemy["vy"] = 0
		enemy["on_ground"] = false
	else:
		if int(enemy["vy"]) != 0:
			enemy["on_ground"] = false

func _bump_goombas() -> void:
	var count := enemies.size()
	for i in count:
		var left: Dictionary = enemies[i]
		if not bool(left["alive"]) or not bool(left["awake"]):
			continue
		for j in count:
			if i == j:
				continue
			var right: Dictionary = enemies[j]
			if not bool(right["alive"]):
				continue
			if not _overlaps(left, right):
				continue
			var toward: bool = (int(left["vx"]) > 0 and int(right["x"]) >= int(left["x"])) or (int(left["vx"]) < 0 and int(right["x"]) <= int(left["x"]))
			if toward:
				var face := -signum(int(left["facing"]))
				if face == 0:
					face = -1
				left["facing"] = face
				left["vx"] = sat_mul(face, walker_vx)
			break

func _contacts(pad: Dictionary, prev_y: int, prev_vy: int) -> Dictionary:
	var player := _player_rect()
	var stomped := false
	var hurt := false
	for enemy in enemies:
		if not bool(enemy["alive"]) or not bool(enemy["awake"]):
			continue
		if not _overlaps(player, enemy):
			continue
		var from_above: bool = prev_vy > 0 and sat_add(prev_y, body_h) <= sat_add(int(enemy["y"]), idiv(int(enemy["h"]), 2))
		if from_above:
			enemy["alive"] = false
			stomped = true
		else:
			hurt = true
	if stomped:
		if bool(pad["jump_held"]):
			body_vy = stomp_vy_held
		else:
			body_vy = stomp_vy
		on_ground = false
		is_jumping = true
	if hurt:
		_defeat()
	return {"stomp": stomped, "hurt": hurt}

func _goals() -> bool:
	var player := _player_rect()
	for goal in goals:
		if _overlaps(player, goal):
			won = true
			body_vx = 0
			body_vy = 0
			return true
	return false

func _defeat() -> void:
	body_w = hit_w
	body_h = hit_h
	body_x = spawn_x
	body_y = spawn_y
	body_vx = 0
	body_vy = 0
	on_ground = true
	is_jumping = false
	lives = maxi(lives - 1, 0)

func _update_camera() -> void:
	var max_x := maxi(level_w - screen_w_fp, 0)
	var max_y := maxi(level_h - screen_h_fp, 0)
	var focus_x := body_x - idiv(screen_w_fp * 35, 100)
	var focus_y := body_y - idiv(screen_h_fp * 55, 100)
	camera_x = clampi(focus_x, 0, max_x)
	camera_y = clampi(focus_y, 0, max_y)

func _sees(x: int, y: int, w: int, h: int) -> bool:
	var left := camera_x - view_margin
	var top := camera_y - view_margin
	var right := camera_x + screen_w_fp + view_margin
	var bottom := camera_y + screen_h_fp + view_margin
	return x < right and sat_add(x, w) > left and y < bottom and sat_add(y, h) > top

func _player_rect() -> Dictionary:
	return {"x": body_x, "y": body_y, "w": body_w, "h": body_h}

func _on_floor(body: Dictionary) -> bool:
	var inset := maxi(idiv(int(body["w"]), 8), fp)
	var probe := {
		"x": sat_add(int(body["x"]), inset),
		"y": sat_add(int(body["y"]), int(body["h"])),
		"w": maxi(int(body["w"]) - sat_mul(inset, 2), fp),
		"h": idiv(fp, 2),
	}
	for solid in solids:
		if bool(solid["broken"]):
			continue
		if _overlaps(probe, solid):
			return true
	return false

func _move_body(body: Dictionary, vx: int, vy: int, corner: bool) -> Dictionary:
	var hit := {"wall": false, "y": "none", "ceiling": -1, "floor": -1}
	if vx != 0:
		body["x"] = sat_add(int(body["x"]), vx)
		for _pass in 4:
			var touched := false
			for solid in solids:
				if bool(solid["broken"]) or not _overlaps(body, solid):
					continue
				touched = true
				hit["wall"] = true
				if vx > 0:
					body["x"] = sat_sub(int(solid["x"]), int(body["w"]))
				else:
					body["x"] = sat_add(int(solid["x"]), tile_fp)
			if not touched:
				break
	if vy == 0:
		return hit
	body["y"] = sat_add(int(body["y"]), vy)
	if corner and vy < 0:
		_apply_corner_nudge(body)
	for _pass in 4:
		var touched := false
		var index := 0
		for solid in solids:
			if bool(solid["broken"]) or not _overlaps(body, solid):
				index += 1
				continue
			if vy < 0 and corner:
				var nudge: Variant = _corner_nudge(body, solid)
				if nudge != null:
					index += 1
					continue
			touched = true
			if vy > 0:
				body["y"] = sat_sub(int(solid["y"]), int(body["h"]))
				hit["y"] = "floor"
				hit["floor"] = index
			else:
				body["y"] = sat_add(int(solid["y"]), tile_fp)
				hit["y"] = "ceiling"
				hit["ceiling"] = index
			index += 1
		if not touched:
			break
	return hit

func _apply_corner_nudge(body: Dictionary) -> void:
	for solid in solids:
		if bool(solid["broken"]) or not _overlaps(body, solid):
			continue
		var nudge: Variant = _corner_nudge(body, solid)
		if nudge != null:
			body["x"] = sat_add(int(body["x"]), int(nudge))
			return

func _corner_nudge(body: Dictionary, solid: Dictionary) -> Variant:
	var into_left := sat_sub(sat_add(int(body["x"]), int(body["w"])), int(solid["x"]))
	var into_right := sat_sub(sat_add(int(solid["x"]), tile_fp), int(body["x"]))
	if into_left > 0 and into_left <= corner_fp and into_left < into_right:
		if not _occupied(int(solid["cell_x"]) - 1, int(solid["cell_y"])):
			return -nudge_fp
	if into_right > 0 and into_right <= corner_fp and into_right < into_left:
		if not _occupied(int(solid["cell_x"]) + 1, int(solid["cell_y"])):
			return nudge_fp
	return null

func _occupied(cell_x: int, cell_y: int) -> bool:
	for solid in solids:
		if bool(solid["broken"]):
			continue
		if int(solid["cell_x"]) == cell_x and int(solid["cell_y"]) == cell_y:
			return true
	return false

func _overlaps(a: Dictionary, b: Dictionary) -> bool:
	var ax := int(a["x"])
	var ay := int(a["y"])
	var ar := sat_add(ax, int(a["w"]))
	var ab := sat_add(ay, int(a["h"]))
	var bx := int(b["x"])
	var by := int(b["y"])
	var br := sat_add(bx, int(b["w"]))
	var bb := sat_add(by, int(b["h"]))
	return ax < br and ar > bx and ay < bb and ab > by

func approach_zero(vx: int, decel: int) -> int:
	if vx > 0:
		if vx <= decel:
			return 0
		return vx - decel
	if vx < 0:
		if vx >= -decel:
			return 0
		return vx + decel
	return 0

func pull_toward(vx: int, limit: int, decel: int) -> int:
	if vx > limit:
		var next := vx - decel
		if next < limit:
			return limit
		return next
	if vx < -limit:
		var next := vx + decel
		if next > -limit:
			return -limit
		return next
	return vx

func sat_add(a: int, b: int) -> int:
	var sum := a + b
	if sum > I32_MAX:
		return I32_MAX
	if sum < I32_MIN:
		return I32_MIN
	return sum

func sat_sub(a: int, b: int) -> int:
	var diff := a - b
	if diff > I32_MAX:
		return I32_MAX
	if diff < I32_MIN:
		return I32_MIN
	return diff

func sat_mul(a: int, b: int) -> int:
	var product := a * b
	if product > I32_MAX:
		return I32_MAX
	if product < I32_MIN:
		return I32_MIN
	return product

func iabs(v: int) -> int:
	if v < 0:
		return -v
	return v

func signum(v: int) -> int:
	if v > 0:
		return 1
	if v < 0:
		return -1
	return 0

func imin(a: int, b: int) -> int:
	if a < b:
		return a
	return b

func idiv(a: int, b: int) -> int:
	return int(float(a) / float(b))

func _cmp_int(label: String, actual: int, raw: Variant) -> String:
	var expected := int(raw)
	if actual != expected:
		return "%s %s != %s" % [label, actual, expected]
	return ""

func _cmp_bool(label: String, actual: bool, raw: Variant) -> String:
	var expected := bool(raw)
	if actual != expected:
		return "%s %s != %s" % [label, actual, expected]
	return ""
