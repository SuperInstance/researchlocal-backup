--[[
    ROBUX-CONSTRUCT: ShipyardUI [v1.0.0-Alpha]
    Purpose: Client-side interface logic for the Shipyard.
    Functionality: Provides visual feedback, button handling, and credit interaction triggers.
]]

local ShipyardUI = {}
ShipyardUI.__index = ShipyardUI

-- UI Constants / Mock IDs
local UI_ELEMENTS = {
    CONTAINER = "ShipyardFrame",
    CREDIT_DISPLAY = "CreditLabel",
    UPGRADE_LIST = "UpgradeListContainer",
    FEEDBACK_TEXT = "NotificationText"
}

function ShipyardUI.new(shipyard_system)
    local self = setmetatable({}, ShipyardUI)
    self.System = shipyard_system
    self.IsVisible = false
    return self
end

function ShipyardUI:Open()
    self.IsVisible = true
    self:RefreshDisplay()
    print("[UI]: Shipyard Interface Opened.")
end

function ShipyardUI:Close()
    self.IsVisible = false
    print("[UI]: Shipyard Interface Closed.")
end

function ShipyardUI:RefreshDisplay()
    if not self.System or not self.System.PlayerData then
        warn("[UI]: System or PlayerData missing for refresh.")
        return
    end

    -- Mock: Update Credit Display
    print(string.format("[UI]: Displaying Credits: %d", self.System.PlayerData.credits))
    
    -- Mock: Render Upgrade List
    print("[UI]: Rendering Upgrade List from Manifest...")
    for id, info in pairs(self.System.Manifest.upgrades) do
        print(string.format("  - [%s] %s (Cost: %d)", id, info.name, info.levels[1].cost))
    end
end

function ShipyardUI:OnUpgradeClicked(upgrade_id, level)
    print(string.format("[UI]: Upgrade Clicked: %s at Level %d", upgrade_id, level))
    
    local success = self.System:PurchaseUpgrade(upgrade_id, level)
    
    if success then
        self:ShowFeedback("UPGRADE SUCCESSFUL!", "Green")
    else
        self:ShowFeedback("INSUFFICIENT CREDITS", "Red")
    end
    
    self:RefreshDisplay()
end

function ShipyardUI:ShowFeedback(message, color)
    print(string.format("[UI-FEEDBACK] [%s]: %s", color, message))
end

-- Mock event listener for testing
function ShipyardUI:SimulateClick(upgrade_id, level)
    self:OnUpgradeClicked(upgrade_id, level)
end

return ShipyardUI
