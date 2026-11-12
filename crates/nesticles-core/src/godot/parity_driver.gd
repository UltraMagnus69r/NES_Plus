extends Node

# Replays Rust tick_play traces. This does nothing unless Godot was started
# with --nesticles-parity. Expected states are data. The step lives in nesticles_sim.gd.

const SimScript := preload("res://scripts/nesticles_sim.gd")

func _ready() -> void:
	if not _requested():
		return
	if not FileAccess.file_exists("res://data/parity.json"):
		_fail("parity data is missing", "", 0)
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string("res://data/parity.json"))
	if typeof(parsed) != TYPE_DICTIONARY:
		_fail("parity data is not a dictionary", "", 0)
		return
	var data: Dictionary = parsed
	var cases: Array = data["cases"]
	var total := 0
	for case_data in cases:
		var case_name := str(case_data["name"])
		var sim = SimScript.new()
		var loaded: String = sim.load_state(case_data["state"])
		if loaded != "":
			_fail(loaded, case_name, 0)
			return
		var ticks: Array = case_data["ticks"]
		var expect: Array = case_data["expect"]
		if ticks.size() != expect.size():
			_fail("tick count does not match the expected states", case_name, 0)
			return
		for index in ticks.size():
			sim.step(ticks[index])
			var err: String = sim.difference(expect[index])
			if err != "":
				_fail(err, case_name, index + 1)
				return
			total += 1
	print("NESTICLES_PARITY_OK cases=%s ticks=%s" % [cases.size(), total])
	get_tree().quit(0)

func _requested() -> bool:
	for arg in OS.get_cmdline_user_args():
		if str(arg) == "--nesticles-parity":
			return true
	for arg in OS.get_cmdline_args():
		if str(arg) == "--nesticles-parity":
			return true
	return false

func _fail(reason: String, case_name: String, tick: int) -> void:
	print("NESTICLES_PARITY_FAIL case=%s tick=%s %s" % [case_name, tick, reason])
	get_tree().quit(1)
