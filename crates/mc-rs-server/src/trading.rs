//! Trading — port conceptuel de PMMP `src/entity/Villager.php` + trade system.
//! PMMP a un système villager-trade partiel ; ici on modélise MerchantOffer
//! + MerchantRecipe lists pour futur villager IA.

use mc_rs_proto::packets::player::ItemStack;

#[derive(Debug, Clone)]
pub struct MerchantOffer {
    pub buy_a: ItemStack,
    pub buy_b: Option<ItemStack>, // secondaire
    pub sell: ItemStack,
    pub uses: u32,
    pub max_uses: u32,
    pub xp_given_to_player: u32,
    pub xp_given_to_villager: u32,
    pub price_multiplier: f32,
    /// Required villager level (Novice=1, Apprentice=2, ...).
    pub required_level: u8,
}

impl MerchantOffer {
    pub fn is_disabled(&self) -> bool {
        self.uses >= self.max_uses
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillagerProfession {
    Unemployed,
    Farmer,
    Fisherman,
    Shepherd,
    Fletcher,
    Librarian,
    Cartographer,
    Cleric,
    Armorer,
    WeaponSmith,
    ToolSmith,
    Butcher,
    Leatherworker,
    Mason,
    Nitwit,
}

impl VillagerProfession {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Unemployed => "none",
            Self::Farmer => "farmer",
            Self::Fisherman => "fisherman",
            Self::Shepherd => "shepherd",
            Self::Fletcher => "fletcher",
            Self::Librarian => "librarian",
            Self::Cartographer => "cartographer",
            Self::Cleric => "cleric",
            Self::Armorer => "armor",
            Self::WeaponSmith => "weapon",
            Self::ToolSmith => "tool",
            Self::Butcher => "butcher",
            Self::Leatherworker => "leather",
            Self::Mason => "mason",
            Self::Nitwit => "nitwit",
        }
    }
}

#[derive(Debug, Clone)]
pub struct VillagerTradeList {
    pub profession: VillagerProfession,
    pub level: u8, // 1=Novice 5=Master
    pub offers: Vec<MerchantOffer>,
}

impl VillagerTradeList {
    pub fn new(profession: VillagerProfession) -> Self {
        Self {
            profession,
            level: 1,
            offers: Vec::new(),
        }
    }

    pub fn available_offers(&self) -> impl Iterator<Item = &MerchantOffer> {
        self.offers
            .iter()
            .filter(|o| !o.is_disabled() && o.required_level <= self.level)
    }

    pub fn add(&mut self, offer: MerchantOffer) {
        self.offers.push(offer);
    }

    /// Consomme un offer (uses += 1).
    pub fn use_offer(&mut self, index: usize) -> Option<&MerchantOffer> {
        let o = self.offers.get_mut(index)?;
        if o.is_disabled() {
            return None;
        }
        o.uses += 1;
        Some(o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: i32, count: u16) -> ItemStack {
        ItemStack::new(id, count, 0)
    }

    #[test]
    fn disabled_when_max_uses_reached() {
        let offer = MerchantOffer {
            buy_a: stack(1, 1),
            buy_b: None,
            sell: stack(2, 1),
            uses: 12,
            max_uses: 12,
            xp_given_to_player: 3,
            xp_given_to_villager: 5,
            price_multiplier: 0.05,
            required_level: 1,
        };
        assert!(offer.is_disabled());
    }

    #[test]
    fn available_offers_respects_level() {
        let mut tl = VillagerTradeList::new(VillagerProfession::Farmer);
        tl.level = 1;
        tl.add(MerchantOffer {
            buy_a: stack(1, 1),
            buy_b: None,
            sell: stack(2, 1),
            uses: 0,
            max_uses: 12,
            xp_given_to_player: 1,
            xp_given_to_villager: 1,
            price_multiplier: 0.05,
            required_level: 1,
        });
        tl.add(MerchantOffer {
            buy_a: stack(3, 1),
            buy_b: None,
            sell: stack(4, 1),
            uses: 0,
            max_uses: 12,
            xp_given_to_player: 2,
            xp_given_to_villager: 2,
            price_multiplier: 0.05,
            required_level: 3,
        });
        assert_eq!(tl.available_offers().count(), 1);
        tl.level = 3;
        assert_eq!(tl.available_offers().count(), 2);
    }
}
