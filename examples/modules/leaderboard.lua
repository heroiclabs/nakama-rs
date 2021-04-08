local nk = require("nakama")

nk.run_once(function(context)
	nk.leaderboard_create("wins", false, "desc", "incr")
end)
