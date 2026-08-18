--[[
Hermes Roblox Construct - General Automation Template
__PROJECT_NAME__
]]--

local Hermes = require(script.Parent.Shared.init)

-- General Automation Configuration
local AutomationAgent = {
    CustomTasks = {},
    TaskQueue = {},
    TaskInterval = 3 -- Seconds between task queue checks
}

-- Initialize Automation Agent
function AutomationAgent:Init()
    Hermes:Init()
    print("🤖 Starting General Automation Agent: __PROJECT_NAME__")
    
    -- Load custom voice commands from Foreman
    if Hermes.STTEnabled then
        SuperInstance.Foreman:RegisterCommand("add task", function(taskName)
            table.insert(self.TaskQueue, taskName)
            return string.format("✅ Added task: %s", taskName)
        end)
        
        SuperInstance.Foreman:RegisterCommand("clear tasks", function()
            self.TaskQueue = {}
            return "✅ Cleared all tasks"
        end)
    end
    
    -- Start automated task loop
    Hermes:RunTask(function()
        self:RunAutomationCycle()
    end)
end

-- Run One Full Automation Cycle
function AutomationAgent:RunAutomationCycle()
    if #self.TaskQueue == 0 then
        if Hermes.TTSEnabled then
            SuperInstance.Foreman:Say("ℹ️  No queued tasks, waiting for commands")
        end
        wait(self.TaskInterval)
        return
    end
    
    local currentTask = table.remove(self.TaskQueue, 1)
    
    -- Execute custom task
    local success, err = pcall(function()
        -- Example task execution logic
        if currentTask == "farm resources" then
            self:RunResourceFarm()
        elseif currentTask == "build base" then
            self:RunBaseBuild()
        elseif currentTask == "patrol area" then
            self:RunPatrol()
        else
            error(string.format("Unknown task: %s", currentTask))
        end
    end)
    
    if not success then
        if Hermes.TTSEnabled then
            SuperInstance.Foreman:Say(string.format("❌ Task failed: %s", err))
        end
        print(string.format("❌ Task failed: %s", err))
    end
    
    -- Log task to SuperInstance vector index
    if Hermes.VectorSyncEnabled then
        SuperInstance.VectorIndex:Ingest({
            type = "automation-task",
            task = currentTask,
            success = success,
            timestamp = os.time()
        })
    end
end

-- Helper Functions
function AutomationAgent:RunResourceFarm()
    -- Reusable resource farming logic
    local nearbyResources = workspace:GetDescendantsWhere(function(child)
        return child:IsA("ClickDetector")
    end)
    for _, resource in ipairs(nearbyResources) do
        LocalPlayer.Character:MoveTo(resource.Parent.Position)
        fireclickdetector(resource)
        wait(1.5)
    end
end

function AutomationAgent:RunBaseBuild()
    -- Reusable base building logic
    local buildSpots = workspace.BaseTerrain:GetDescendantsWhere(function(child)
        return child:IsA("TerrainPoint") and child.Flattened
    end)
    for _, spot in ipairs(buildSpots) do
        local newStructure = Instance.new("Wall")
        newStructure.Position = spot.Position
        newStructure.Parent = workspace
        wait(2)
    end
end

function AutomationAgent:RunPatrol()
    -- Reusable patrol logic
    local patrolPoints = workspace.PatrolPoints:GetChildren()
    for _, point in ipairs(patrolPoints) do
        LocalPlayer.Character:MoveTo(point.Position)
        wait(3)
    end
end

-- Start Agent
AutomationAgent:Init()