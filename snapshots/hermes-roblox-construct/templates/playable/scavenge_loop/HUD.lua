--[[
    ROBUX-CONSTRUCT: HUD [v1.0.0-Alpha]
    Purpose: Real-time player status and resource HUD.
]]

local HUD = {}
HUD.__index = HUD

function HUD.new(player_data)
    local self = setmetatable({}, HUD)
    self.PlayerData = player_data
    return self
end

function HUD:Update()
    -- This would normally interact with Roblox UI components
    -- Mocking the update cycle
    self:Render()
end

function HUD:Render()
    -- Representing the visual readout in logs for the prototype
    local credits = self.PlayerData.credits or 0
    local hull = self.PlayerData.hull or 100
    
    -- Simple visual strip
    -- -----------------------------------------
    -- | Credits: 500 | Hull: [||||------] 50% |
    -- -----------------------------------------
    local hull_bar = self:GenerateHullBar(hull)
    print(string.format("[HUD] Credits: %d | Hull: %s (%d%%)", credits, hull_bar, hull))
end

function HUD:GenerateHullBar(percent)
    local segments = 10
    local filled = math.floor((percent / 100) * segments)
    local empty = segments - filled
    return string.rep("|", filled) .. string.rep("-", empty)
end

return HUD
