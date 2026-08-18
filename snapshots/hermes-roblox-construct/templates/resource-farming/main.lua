--[[
Hermes Roblox Construct - Resource Farming Template
__PROJECT_NAME__
]]--

local Hermes = require(script.Parent.Shared.init)

-- Resource Farming Configuration
local ResourceFarming = {
    TargetResources = {"Wood", "Stone", "Iron"},
    FarmRadius = 30,
    TaskInterval = 2 -- Seconds between task checks
}

-- Initialize Resource Farming Agent
function ResourceFarming:Init()
    Hermes:Init()
    print("🌾 Starting Resource Farming Agent: __PROJECT_NAME__")
    
    -- Start automated task loop
    Hermes:RunTask(function()
        self:RunFarmCycle()
    end)
end

-- Run One Full Farm Cycle
function ResourceFarming:RunFarmCycle()
    --Scan for nearby resources
    local nearbyResources = workspace:GetDescendantsWhere(function(child)
        return child:IsA("ClickDetector") and table.find(self.TargetResources, child.Name)
    end)
    
    for _, resource in ipairs(nearbyResources) do
        if #Players:GetPlayers() > 0 then break end -- Skip if player is present
        
        -- Move to resource
        LocalPlayer.Character:MoveTo(resource.Parent.Position)
        wait(0.5)
        
        -- Mine resource
        fireclickdetector(resource)
        wait(1.5)
        
        -- Log task to SuperInstance vector index
        if Hermes.VectorSyncEnabled then
            SuperInstance.VectorIndex:Ingest({
                type = "resource-farm",
                resource = resource.Name,
                position = resource.Parent.Position,
                timestamp = os.time()
            })
        end
    end
    
    -- Send status update via Foreman TTS
    if Hermes.TTSEnabled then
        SuperInstance.Foreman:Say(string.format(
            "✅ Farm cycle complete: Collected %d resources", 
            #LocalPlayer.Backpack:GetChildren()
        ))
    end
end

-- Start Agent
ResourceFarming:Init()