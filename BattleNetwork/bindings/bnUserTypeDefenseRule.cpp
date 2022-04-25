#ifdef ONB_MOD_SUPPORT
#include "bnUserTypeDefenseRule.h"

#include "bnScopedWrapper.h"
#include "bnWeakWrapper.h"
#include "bnScriptedDefenseRule.h"
#include "../bnDefenseVirusBody.h"
#include "../bnSolHelpers.h"

void DefineDefenseRuleUserTypes(sol::state& state, sol::table& battle_namespace) {
  battle_namespace.new_usertype<WeakWrapper<ScriptedDefenseRule>>("DefenseRule",
    sol::factories(
      [](DefensePriority priority, const DefenseOrder& order) -> WeakWrapper<ScriptedDefenseRule> {
        if (priority == DefensePriority::Internal) {
          std::runtime_error("DefensePriority reserved for internal use");
        }

        auto defenseRule = std::make_shared<ScriptedDefenseRule>(priority, order);

        auto wrappedRule = WeakWrapper(defenseRule);
        wrappedRule.Own();
        return wrappedRule;
      }
    ),
    sol::meta_function::index, [](WeakWrapper<ScriptedDefenseRule>& defenseRule, const std::string& key) {
      return defenseRule.Unwrap()->dynamic_get(key);
    },
    sol::meta_function::new_index, [](WeakWrapper<ScriptedDefenseRule>& defenseRule, const std::string& key, sol::stack_object value) {
      defenseRule.Unwrap()->dynamic_set(key, value);
    },
    sol::meta_function::length, [](WeakWrapper<ScriptedDefenseRule>& defenseRule) {
      return defenseRule.Unwrap()->entries.size();
    },
    "is_replaced", [] (WeakWrapper<ScriptedDefenseRule>& defenseRule) -> bool {
      auto ptr = defenseRule.Lock();
      return ptr == nullptr || ptr->IsReplaced();
    },
    "filter_statuses_func", sol::property(
      [](WeakWrapper<ScriptedDefenseRule>& defenseRule) { return defenseRule.Unwrap()->filter_statuses_func; },
      [](WeakWrapper<ScriptedDefenseRule>& defenseRule, sol::stack_object value) {
        defenseRule.Unwrap()->filter_statuses_func = VerifyLuaCallback(value);
      }
    ),
    "can_block_func", sol::property(
      [](WeakWrapper<ScriptedDefenseRule>& defenseRule) { return defenseRule.Unwrap()->can_block_func; },
      [](WeakWrapper<ScriptedDefenseRule>& defenseRule, sol::stack_object value) {
        defenseRule.Unwrap()->can_block_func = VerifyLuaCallback(value);
      }
    ),
    "replace_func", sol::property(
      [](WeakWrapper<ScriptedDefenseRule>& defenseRule) { return defenseRule.Unwrap()->replace_func; },
      [](WeakWrapper<ScriptedDefenseRule>& defenseRule, sol::stack_object value) {
        defenseRule.Unwrap()->replace_func = VerifyLuaCallback(value);
      }
    )
  );

  battle_namespace.new_usertype<DefenseVirusBody>("DefenseVirusBody",
    sol::factories(
      [] () -> WeakWrapper<DefenseRule> {
        std::shared_ptr<DefenseRule> defenseRule = std::make_shared<DefenseVirusBody>();

        auto wrappedRule = WeakWrapper(defenseRule);
        wrappedRule.Own();
        return wrappedRule;
      }
    )
  );

  battle_namespace.new_usertype<ScopedWrapper<DefenseFrameStateJudge>>("DefenseFrameStateJudge",
    "block_damage", [](ScopedWrapper<DefenseFrameStateJudge>& judge) {
      judge.Unwrap().BlockDamage();
    },
    "block_impact", [](ScopedWrapper<DefenseFrameStateJudge>& judge) {
      judge.Unwrap().BlockImpact();
    },
    "is_damage_blocked", [](ScopedWrapper<DefenseFrameStateJudge>& judge) -> bool {
      return judge.Unwrap().IsDamageBlocked();
    },
    "is_impact_blocked", [](ScopedWrapper<DefenseFrameStateJudge>& judge) -> bool {
      return judge.Unwrap().IsImpactBlocked();
    },
    /*"add_trigger", &DefenseFrameStateJudge::AddTrigger,*/
    "signal_defense_was_pierced", [](ScopedWrapper<DefenseFrameStateJudge>& judge) {
      judge.Unwrap().SignalDefenseWasPierced();
    }
  );

  state.new_enum("DefenseOrder",
    "Always", DefenseOrder::always,
    "CollisionOnly", DefenseOrder::collisionOnly
  );

  state.new_enum("DefensePriority",
    // "Internal", DefensePriority::Internal, // internal use only
    // "Passthrough", DefensePriority::Passthrough, // excluded as modders should use set_passthrough
    "Barrier", DefensePriority::Barrier,
    "Body", DefensePriority::Body,
    "CardAction", DefensePriority::CardAction,
    "Trap", DefensePriority::Trap,
    "Last", DefensePriority::Last // special case, appends instead of replaces
  );
}
#endif
