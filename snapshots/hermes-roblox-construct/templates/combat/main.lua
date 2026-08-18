--[[
Hermes Roblox Construct - Combat Agent Template
__PROJECT_NAME__
]]--

local Hermes = require(script.Parent.Shared.init)

-- Combat Configuration
local CombatAgent = {
    TargetMobs = {"Zombie", "Skeleton", "Spider"},
    CombatRadius = 40,
    HealthThreshold = 20,
    TaskInterval = 1 -- Seconds between combat checks
}

-- Initialize Combat Agent
function CombatAgent:Init()
    Hermes:Init()
    print("⚔️  Starting Combat Agent: __PROJECT_NAME__")
    
    -- Start automated combat loop
    Hermes:RunTask(function()
        self:RunCombatCycle()
    end)
end

-- Run One Full Combat Cycle
function CombatAgent:RunCombatCycle()
    -- Check player health
    local humanoid = LocalPlayer.Character:WaitForChild("Humanoid")
    if humanoid.Health < self.HealthThreshold then
        if Hermes.TTSEnabled then
            SuperInstance.Foreman:Say("⚠️  Low health, retreating to base")
        end
        LocalPlayer.Character:MoveTo(workspace.Base.Position)
        return
    end
    
    -- Scan for nearby mobs
    local nearbyMobs = workspace:GetDescendantsWhere(function(child)
        return child:IsA("Humanoid") and table.find(self.TargetMobs, child.Name)
    end)
    
    for _, mob in ipairs(nearbyMobs) do
        -- Attack mob
        LocalPlayer.Character:MoveTo(mob.Parent.Position)
        wait(0.5)
        LocalPlayer.Character:FindFirstChild("Tool"):Activate()
        wait(1.2)
        
        -- Log task to SuperInstance vector index
        if Hermes.VectorSyncEnabled then
            SuperInstance.VectorIndex:Ingest({
                type = "combat",
                mob = mob.Name,
                position = mob.Parent.Position,
                timestamp = os.time()
            })
        end
    end
    
    -- Send status update via Foreman TTS
    local kills = #workspace:GetDescendantsWhere(function(child)
        return child:IsA("Explosion") and child.Creator == LocalPlayer
    end)
    if Hermes.TTSEnabled then
        SuperInstance.Foreman:Say(string.format(
            "✅ Combat cycle complete: Eliminated %d enemies", 
            kills
        ))
    end
end

-- Start Agent
CombatAgent:Init()