--[[
    ROBUX-CONSTRUCT: ShipyardSystem [v1.1.0-Alpha]
    Purpose: The economic backbone. Manages Credits and applying upgrades.
    Mechanism: Manifest-driven upgrade application with multiplier application.
]]

local ShipyardSystem = {}
ShipyardSystem.__index = ShipyardSystem

function ShipyardSystem.new(manifest_path, player_data)
    local self = setmetatable({}, ShipyardSystem)
    self.Manifest = nil -- To be loaded from JSON
    self.PlayerData = player_data -- {credits = 0, upgrades = {}, stats = {hull = 100, cargo = 50, speed = 10}}
    self.ManifestPath = manifest_path
    return self
end

function ShipyardSystem:LoadManifest(manifest_data)
    self.Manifest = manifest_data
    print("[SHIPYARD]: Manifest loaded into memory.")
end

function ShipyardSystem:PurchaseUpgrade(upgrade_id, level)
    if not self.Manifest or not self.Manifest.upgrades[upgrade_id] then
        print("[SHIPYARD]: Error - Unknown Upgrade ID: " .. tostring(upgrade_id))
        return false
    end

    local upgrade_info = self.Manifest.upgrades[upgrade_id].levels[level]
    if not upgrade_info then
        print("[SHIPYARD]: Error - Invalid Upgrade Level: " .. level)
        return false
    end

    local cost = upgrade_info.cost

    if self.PlayerData.credits >= cost then
        self.PlayerData.credits -= cost
        
        -- Update level tracking
        self.PlayerData.upgrades[upgrade_id] = (self.PlayerData.upgrades[upgrade_id] or 0) + 1
        
        -- Apply the multiplier to actual player stats
        local multiplier = upgrade_info.multiplier or 1.0
        self:ApplyStatMultiplier(upgrade_id, multiplier)
        
        print("[SHIPYARD]: Purchase Successful! " .. upgrade_id .. " Lvl " .. level .. ". New Balance: " .. self.PlayerData.credits)
        return true
    else
        print("[SHIPYARD]: Insufficient Credits! Required: " .. cost .. ", Available: " .. self.PlayerData.credits)
        return false
    end
end

function ShipyardSystem:ApplyStatMultiplier(upgrade_id, multiplier)
    if not self.PlayerData.stats then 
        self.PlayerData.stats = {} 
    end

    -- Mapping IDs to stat keys
    local mapping = {
        hull_integrity = "hull",
        cargo_capacity = "cargo",
        swarm_efficiency = "speed"
    }

    local stat_key = mapping[upgrade_id]
    if stat_key and self.PlayerData.stats[stat_key] then
        self.PlayerData.stats[stat_key] = self.PlayerData.stats[stat_key] * multiplier
        print(string.format("[SHIPYARD]: Applied multiplier %.2f to %s. New value: %.2f", multiplier, stat_key, self.PlayerData.stats[stat_key]))
    else
        print("[SHIPYARD]: Warning - No stat mapping found for " .. upgrade_id)
    end
end

return ShipyardSystem
