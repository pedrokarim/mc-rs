# 35 – Codes des écrans d'erreur du client

Quand le client Bedrock se déconnecte, il affiche un mot-code (« Block », « Boat »…) et un détail
`InitialConnection-NNN`, où NNN est l'indice de la cause dans l'énumération Mojang `DisconnectFailReason`
(`.reference/bedrock-protocol-docs/json/DisconnectFailReason.json`).

La table ci-dessous associe chaque cause à son mot-code. Elle est extraite de BDS 1.26.52.3
(`definitions/disconnection_errors/disconnection_error_messaging.json`, thèmes et surcharges aplatis ; repli :
« Terracotta ») et indexée sur l'énumération de la release 1.26.51. Les causes absentes du fichier affichent
le repli.

Causes déjà rencontrées sur mc-rs : `BadPacket` (90, « Block », montée 2169 :
[`31-PROTOCOL-2169.md`](31-PROTOCOL-2169.md) § 5) et `MissingStructureData` (148, « Boat », montée 2193 :
[`34-BDS-RETRO-INGENIERIE.md`](34-BDS-RETRO-INGENIERIE.md) § 7).

| Indice | `DisconnectFailReason` | Mot-code |
|---|---|---|
| 0 | `Unknown` | Creeper |
| 1 | `CantConnectNoInternet` | NetherNet |
| 2 | `NoPermissions` | Spyglass |
| 3 | `UnrecoverableError` | Chest |
| 4 | `ThirdPartyBlocked` | Fox |
| 5 | `ThirdPartyNoInternet` | NetherNet |
| 6 | `ThirdPartyBadIP` | Squid |
| 7 | `ThirdPartyNoServerOrServerLocked` | Silverfish |
| 8 | `VersionMismatch` | Chain |
| 9 | `SkinIssue` | Armor |
| 10 | `InviteSessionNotFound` | NetherNet |
| 11 | `EduLevelSettingsMissing` | Gold |
| 12 | `LocalServerNotFound` | Panda |
| 13 | `LegacyDisconnect` | NetherNet |
| 15 | `PlatformLockedSkinsError` | Armor |
| 16 | `RealmsWorldUnassigned` | Arrow |
| 17 | `RealmsServerCantConnect` | Arrow |
| 18 | `RealmsServerHidden` | Arrow |
| 19 | `RealmsServerDisabledBeta` | Arrow |
| 20 | `RealmsServerDisabled` | Arrow |
| 21 | `CrossPlatformDisabled` | Spyglass |
| 23 | `SessionNotFound` | Goat |
| 25 | `ServerFull` | Hopper |
| 26 | `InvalidPlatformSkin` | Armor |
| 27 | `EditionVersionMismatch` | Chain |
| 28 | `EditionMismatch` | Chain |
| 29 | `LevelNewerThanExeVersion` | Breeze |
| 31 | `BannedSkin` | Armor |
| 32 | `Timeout` | Kelp |
| 33 | `ServerNotFound` | Clay |
| 34 | `OutdatedServer` | Chain |
| 35 | `OutdatedClient` | Chain |
| 37 | `MultiplayerDisabled` | Spyglass |
| 38 | `NoWiFi` | NetherNet |
| 39 | `WorldCorruption` | Chest |
| 40 | `NoReason` | Creeper |
| 41 | `Disconnected` | Bat |
| 42 | `InvalidPlayer` | Guardian |
| 43 | `LoggedInOtherLocation` | Guardian |
| 44 | `ServerIdConflict` | Blaze |
| 45 | `NotAllowed` | Spyglass |
| 46 | `NotAuthenticated` | Guardian |
| 47 | `InvalidTenant` | Gold |
| 48 | `UnknownPacket` | Block |
| 49 | `UnexpectedPacket` | Block |
| 50 | `InvalidCommandRequestPacket` | Block |
| 51 | `HostSuspended` | Boat |
| 52 | `LoginPacketNoRequest` | Guardian |
| 53 | `LoginPacketNoCert` | Guardian |
| 54 | `MissingClient` | Snowball |
| 55 | `Kicked` | Evoker |
| 56 | `KickedForExploit` | Evoker |
| 57 | `KickedForIdle` | Evoker |
| 58 | `ResourcePackProblem` | Rabbit |
| 59 | `IncompatiblePack` | Rabbit |
| 60 | `OutOfStorage` | Chest |
| 61 | `InvalidLevel` | Boat |
| 63 | `BlockMismatch` | Boat |
| 64 | `InvalidHeights` | Lead |
| 65 | `InvalidWidths` | Ladder |
| 66 | `ConnectionLost` | Terracotta |
| 68 | `Shutdown` | Cobweb |
| 70 | `LoadingStateTimeout` | Boat |
| 71 | `ResourcePackLoadingFailed` | Rabbit |
| 72 | `SearchingForSessionLoadingScreenFailed` | Compass |
| 73 | `NetherNetProtocolVersion` | NetherNet |
| 74 | `SubsystemStatusError` | Echo Shard |
| 75 | `EmptyAuthFromDiscovery` | Guardian |
| 76 | `EmptyUrlFromDiscovery` | TNT |
| 77 | `ExpiredAuthFromDiscovery` | Guardian |
| 78 | `UnknownSignalServiceSignInFailure` | Door |
| 79 | `XBLJoinLobbyFailure` | Boat |
| 80 | `UnspecifiedClientInstanceDisconnection` | Feather |
| 81 | `NetherNetSessionNotFound` | NetherNet |
| 82 | `NetherNetCreatePeerConnection` | NetherNet |
| 83 | `NetherNetICE` | Door |
| 84 | `NetherNetConnectRequest` | NetherNet |
| 85 | `NetherNetConnectResponse` | NetherNet |
| 86 | `NetherNetNegotiationTimeout` | Door |
| 87 | `NetherNetInactivityTimeout` | NetherNet |
| 88 | `StaleConnectionBeingReplaced` | NetherNet |
| 90 | `BadPacket` | Block |
| 91 | `NetherNetFailedToCreateOffer` | NetherNet |
| 92 | `NetherNetFailedToCreateAnswer` | NetherNet |
| 93 | `NetherNetFailedToSetLocalDescription` | NetherNet |
| 94 | `NetherNetFailedToSetRemoteDescription` | NetherNet |
| 95 | `NetherNetNegotiationTimeoutWaitingForResponse` | NetherNet |
| 96 | `NetherNetNegotiationTimeoutWaitingForAccept` | NetherNet |
| 97 | `NetherNetIncomingConnectionIgnored` | NetherNet |
| 98 | `NetherNetSignalingParsingFailure` | Door |
| 99 | `NetherNetSignalingUnknownError` | Door |
| 100 | `NetherNetSignalingUnicastDeliveryFailed` | Door |
| 101 | `NetherNetSignalingBroadcastDeliveryFailed` | Door |
| 102 | `NetherNetSignalingGenericDeliveryFailed` | Door |
| 103 | `EditorMismatchEditorWorld` | Emerald |
| 104 | `EditorMismatchVanillaWorld` | Emerald |
| 105 | `WorldTransferNotPrimaryClient` | Block |
| 107 | `ClientGameSetupCancelled` | Boat |
| 108 | `ClientGameSetupFailed` | Boat |
| 110 | `NetherNetSignalingSigninFailed` | Door |
| 111 | `SessionAccessDenied` | Guardian |
| 112 | `ServiceSigninIssue` | Guardian |
| 113 | `NetherNetNoSignalingChannel` | Door |
| 114 | `NetherNetNotLoggedIn` | NetherNet |
| 115 | `NetherNetClientSignalingError` | Door |
| 116 | `SubClientLoginDisabled` | Coal |
| 117 | `DeepLinkTryingToOpenDemoWorldWhileSignedIn` | Honeycomb |
| 118 | `AsyncJoinTaskDenied` | Arrow |
| 119 | `RealmsTimelineRequired` | Arrow |
| 120 | `GuestWithoutHost` | Bamboo |
| 121 | `FailedToJoinExperience` | Terracotta |
| 122 | `NetherNetDataChannelClosed` | NetherNet |
| 123 | `DiscoveryEnvironmentMismatch` | Guardian |
| 124 | `HostWithoutKeys` | Guardian |
| 125 | `HostSignedOut` | Terracotta |
| 126 | `ScriptWatchdogException` | Terracotta |
| 127 | `ScriptMemoryLimitExceeded` | Terracotta |
| 128 | `StorageLowDuringGameplay` | Chest |
| 129 | `StorageFullDuringGameplay` | Chest |
| 130 | `LevelStorageCorruption` | Chest |
| 131 | `EditionMismatchVanillaToEdu` | Chain |
| 132 | `EditionMismatchEduToVanilla` | Chain |
| 133 | `EditorMismatchEditorToVanilla` | Emerald |
| 134 | `EditorMismatchVanillaToEditor` | Emerald |
| 135 | `DenyListed` | Spyglass |
| 136 | `NonceMissing` | Spyglass |
| 137 | `NonceNotFound` | Spyglass |
| 138 | `NonceExpired` | Spyglass |
| 139 | `NonceNotValid` | Spyglass |
| 140 | `HostDisconnected` | Goat |
| 141 | `EditorJoinIntentPolicyFailure` | Emerald |
| 142 | `NetherNetIdentityNotAllowed` | Door |
| 143 | `InvalidName` | Guardian |
| 144 | `ExpiredToken` | Guardian |
| 145 | `HostAcceptsNoTypeOfAuth` | Boat |
| 146 | `NotAuthenticatedFastFail` | Guardian |
| 147 | `EditorNotAllowed` | Emerald |
| 148 | `MissingStructureData` | Boat |
