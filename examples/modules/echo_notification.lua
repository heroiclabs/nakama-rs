local nk = require("nakama")

local function echo(context, payload)
    nk.logger_info(string.format("Payload: %q", payload))

    local content = {
      data = payload,
    }

    nk.notification_send(context.user_id, "Echo", content, 1, context.user_id, true)
end

nk.register_rpc(echo, "echo")
