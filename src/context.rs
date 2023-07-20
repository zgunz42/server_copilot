use std::collections::{HashMap, HashSet};

use teloxide::types::ChatId;
use teloxide::types::Me;
use teloxide::types::UserId;
use crate::gitlab::GitlabUser;

type Address = String;


#[derive(Clone, Debug)]
struct MeBot {
    me: Me,
}

impl Default for MeBot {
    fn default() -> Self {
        Self {
            me: Me {
                can_join_groups: false,
                can_read_all_group_messages: false,
                supports_inline_queries: false,
                user: teloxide::types::User { 
                    id: teloxide::types::UserId(0), 
                    is_bot: false, 
                    first_name: "".to_string(), 
                    last_name: None, 
                    username: None,
                    language_code: None,
                    is_premium: false,
                    added_to_attachment_menu: false,
                }
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Context {
    // map associating several chat IDs to each address
    addr_to_chatids: HashMap<Address, HashSet<ChatId>>,
    // map associating several addresses to each chat ID
    chatid_to_addrs: HashMap<ChatId, HashSet<Address>>,
    // map associating several user IDs to each Gitlab user
    user_to_gitlab: HashMap<UserId, GitlabUser>,
    // current bot
    bot: MeBot,
}

impl Context {
    pub fn new() -> Context {
        Self::default()
    }

    /// returns a bool indicating whether the value was newly inserted
    pub fn register_addr(&mut self, chat_id: ChatId, addr: Address) -> bool {
        let _ = self
            .addr_to_chatids
            .entry(addr.clone())
            .or_default()
            .insert(chat_id);
        self.chatid_to_addrs
            .entry(chat_id)
            .or_default()
            .insert(addr)
    }

    /// returns a bool indicating whether the address was previously registered
    pub fn unregister_addr(&mut self, chat_id: ChatId, addr: Address) -> bool {
        if let Some(chat_ids) = self.addr_to_chatids.get_mut(&addr) {
            let _ = chat_ids.remove(&chat_id);
        };
        match self.chatid_to_addrs.get_mut(&chat_id) {
            Some(addrs) => addrs.remove(&addr),
            None => false,
        }
    }

    /// returns a set of all addresses associated with a chat ID
    pub fn addrs(&self, chat_id: &ChatId) -> Option<&HashSet<Address>> {
        self.chatid_to_addrs.get(chat_id)
    }

    /// returns a set of all chat IDs associated with an address
    pub fn chat_ids<'a>(
        &'a self,
        addr: &Address,
    ) -> Option<&'a HashSet<ChatId>> {
        self.addr_to_chatids.get(addr)
    }

    pub fn set_bot(&mut self, bot: Me) {
        self.bot.me = bot;
    }

    pub fn get_bot(&self) -> &Me {
        &self.bot.me
    }

    pub fn register_gitlab_user(&mut self, user_id: UserId, token: String) {
        let gitlab_user = GitlabUser::new(token);

        self.user_to_gitlab.insert(user_id, gitlab_user);
    }

    pub fn get_gitlab_user(&self, user_id: UserId) -> Option<&GitlabUser> {
        self.user_to_gitlab.get(&user_id)
    }

}