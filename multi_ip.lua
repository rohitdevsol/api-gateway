math.randomseed(os.time())

request = function()
    local ip = "10.0.0." .. math.random(1, 1000)
    local headers = {}
    headers["X-Forwarded-For"] = ip
    return wrk.format(nil, "/api/anything", headers)
end
