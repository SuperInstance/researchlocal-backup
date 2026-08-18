--[[
Hermes Roblox Construct - Base Building Template
__PROJECT_NAME__
]]--

local Hermes = require(script.Parent.Shared.init)

-- Base Building Configuration
local BaseBuilding = {
    BuildStructures = {"Wall", "Turret", "ResourceNode", "Generator"},
    BuildRadius = 50,
    ResourceThreshold = 100,
    TaskInterval = 5 -- Seconds between build checks
}

-- Initialize Base Building Agent
function BaseBuilding:Init()
    Hermes:Init()
    print("🏗️  Starting Base Building Agent: __PROJECT_NAME__")
    
    -- Start automated build loop
    Hermes:RunTask(function()
        self:RunBuildCycle()
    end)
end

-- Run One Full Build Cycle
function BaseBuilding:RunBuildCycle()
    -- Check base resources
    local totalResources = #LocalPlayer.Backpack:GetChildren()
    
    if totalResources < self.ResourceThreshold then
        if Hermes.TTSEnabled then
            SuperInstance.Foreman:Say("⚠️  Low resources, pausing base builds")
        end
        return
    end
    
    -- Find build spot
    local buildSpot = workspace.BaseTerrain:GetDescendantsWhere(function(child)
        return child:IsA("TerrainPoint") and child.Flattened and not child:FindFirstChild("Structure")
    end)
    
    for _, spot in ipairs(buildSpot) do
        -- Pick random structure to build
        local structure = self.BuildStructures[math.random(#self.BuildStructures)]
        
        -- Spawn structure
        local newStructure = Instance.new(structure)
        newStructure.Position = spot.Position
        newStructure.Parent = workspace
        
        -- Log task to SuperInstance vector index
        if Hermes.VectorSyncEnabled then
            SuperInstance.VectorIndex:Ingest({
                type = "base-build",
                structure = structure,
                position = spot.Position,
                timestamp = os.time()
            })
        end
        
        wait(2)
    end
    
    -- Send status update via Foreman TTS
    if Hermes.TTSEnabled then
        SuperInstance.Foreman:Say(string.format(
            "✅ Base build cycle complete: Built %d new structures", 
            #buildSpot
        ))
    end
end

-- Start Agent
BaseBuilding:Init()