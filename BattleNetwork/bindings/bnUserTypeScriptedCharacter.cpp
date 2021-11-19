#ifdef BN_MOD_SUPPORT
#include "bnUserTypeScriptedCharacter.h"

#include "bnWeakWrapper.h"
#include "bnUserTypeAnimation.h"
#include "bnScriptedCharacter.h"
#include "bnScriptedComponent.h"
#include "../bnSolHelpers.h"
#include "../bnTile.h"

void DefineScriptedCharacterUserType(sol::table& battle_namespace) {
  battle_namespace.new_usertype<WeakWrapper<ScriptedCharacter>>("Character",
    sol::meta_function::index, [](WeakWrapper<ScriptedCharacter>& character, const std::string& key) {
      return character.Unwrap()->dynamic_get(key);
    },
    sol::meta_function::new_index, [](WeakWrapper<ScriptedCharacter>& character, const std::string& key, sol::stack_object value) {
      character.Unwrap()->dynamic_set(key, value);
    },
    sol::meta_function::length, [](WeakWrapper<ScriptedCharacter>& character) {
      return character.Unwrap()->entries.size();
    },
    "get_id", [](WeakWrapper<ScriptedCharacter>& character) -> Entity::ID_t {
      return character.Unwrap()->GetID();
    },
    "get_element", [](WeakWrapper<ScriptedCharacter>& character) -> Element {
      return character.Unwrap()->GetElement();
    },
    "set_element", [](WeakWrapper<ScriptedCharacter>& character, Element element) {
      character.Unwrap()->SetElement(element);
    },
    "get_tile", sol::overload(
      [](WeakWrapper<ScriptedCharacter>& character, Direction dir, unsigned count) -> Battle::Tile* {
        return character.Unwrap()->GetTile(dir, count);
      },
      [](WeakWrapper<ScriptedCharacter>& character) -> Battle::Tile* {
        return character.Unwrap()->GetTile();
      }
    ),
    "input_has", [](WeakWrapper<ScriptedCharacter>& character, const InputEvent& event) -> bool {
      return character.Unwrap()->InputState().Has(event);
    },
    "get_current_tile", [](WeakWrapper<ScriptedCharacter>& character) -> Battle::Tile* {
      return character.Unwrap()->GetCurrentTile();
    },
    "get_field", [](WeakWrapper<ScriptedCharacter>& character) -> WeakWrapper<Field> {
      return WeakWrapper(character.Unwrap()->GetField());
    },
    "set_facing", [](WeakWrapper<ScriptedCharacter>& character, Direction dir) {
      character.Unwrap()->SetFacing(dir);
    },
    "get_facing", [](WeakWrapper<ScriptedCharacter>& character) -> Direction {
      return character.Unwrap()->GetFacing();
    },
    "get_facing_away", [](WeakWrapper<ScriptedCharacter>& character) -> Direction {
      return character.Unwrap()->GetFacingAway();
    },
    "get_target", [](WeakWrapper<ScriptedCharacter>& character) -> WeakWrapper<Character> {
      return WeakWrapper(character.Unwrap()->GetTarget());
    },
    "get_color", [](WeakWrapper<ScriptedCharacter>& character) -> sf::Color {
      return character.Unwrap()->getColor();
    },
    "set_color", [](WeakWrapper<ScriptedCharacter>& character, sf::Color color) {
      character.Unwrap()->setColor(color);
    },
    "sprite", [](WeakWrapper<ScriptedCharacter>& character) -> WeakWrapper<SpriteProxyNode> {
      return WeakWrapper(std::static_pointer_cast<SpriteProxyNode>(character.Unwrap()));
    },
    "hide", [](WeakWrapper<ScriptedCharacter>& character) {
      character.Unwrap()->Hide();
    },
    "reveal", [](WeakWrapper<ScriptedCharacter>& character) {
      character.Unwrap()->Reveal();
    },
    "teleport", [](
      WeakWrapper<ScriptedCharacter>& character,
      Battle::Tile* dest,
      ActionOrder order,
      sol::stack_object onBeginObject
    ) -> bool {
      sol::protected_function onBegin = onBeginObject;

      return character.Unwrap()->Teleport(dest, order, [onBegin] {
        auto result = onBegin();

        if (!result.valid()) {
          sol::error error = result;
          Logger::Log(error.what());
        }
      });
    },
    "slide", [](
      WeakWrapper<ScriptedCharacter>& character,
      Battle::Tile* dest,
      const frame_time_t& slideTime,
      const frame_time_t& endlag,
      ActionOrder order,
      sol::stack_object onBeginObject
    ) -> bool {
      sol::protected_function onBegin = onBeginObject;

      return character.Unwrap()->Slide(dest, slideTime, endlag, order, [onBegin] {
        auto result = onBegin();

        if (!result.valid()) {
          sol::error error = result;
          Logger::Log(error.what());
        }
      });
    },
    "jump", [](
      WeakWrapper<ScriptedCharacter>& character,
      Battle::Tile* dest,
      float destHeight,
      const frame_time_t& jumpTime,
      const frame_time_t& endlag,
      ActionOrder order,
      sol::stack_object onBeginObject
    ) -> bool {
      sol::protected_function onBegin = onBeginObject;

      return character.Unwrap()->Jump(dest, destHeight, jumpTime, endlag, order, [onBegin] {
        auto result = onBegin();

        if (!result.valid()) {
          sol::error error = result;
          Logger::Log(error.what());
        }
      });
    },
    "raw_move_event", [](WeakWrapper<ScriptedCharacter>& character, const MoveEvent& event, ActionOrder order) -> bool {
      return character.Unwrap()->RawMoveEvent(event, order);
    },
    "card_action_event", sol::overload(
      [](WeakWrapper<ScriptedCharacter>& character, WeakWrapper<ScriptedCardAction>& cardAction, ActionOrder order) {
        character.Unwrap()->AddAction(CardEvent{ cardAction.Release() }, order);
      },
      [](WeakWrapper<ScriptedCharacter>& character, WeakWrapper<CardAction>& cardAction, ActionOrder order) {
        character.Unwrap()->AddAction(CardEvent{ cardAction.Release() }, order);
      }
    ),
    "is_sliding", [](WeakWrapper<ScriptedCharacter>& character) -> bool {
      return character.Unwrap()->IsSliding();
    },
    "is_jumping", [](WeakWrapper<ScriptedCharacter>& character) -> bool {
      return character.Unwrap()->IsJumping();
    },
    "is_teleporting", [](WeakWrapper<ScriptedCharacter>& character) -> bool {
      return character.Unwrap()->IsTeleporting();
    },
    "is_passthrough", [](WeakWrapper<ScriptedCharacter>& character) -> bool {
      return character.Unwrap()->IsPassthrough();
    },
    "is_moving", [](WeakWrapper<ScriptedCharacter>& character) -> bool {
      return character.Unwrap()->IsMoving();
    },
    "is_deleted", [](WeakWrapper<ScriptedCharacter>& character) -> bool {
      auto ptr = character.Lock();
      return !ptr || ptr->IsDeleted();
    },
    "will_erase_eof", [](WeakWrapper<ScriptedCharacter>& character) -> bool {
      auto ptr = character.Lock();
      return !ptr || ptr->WillEraseEOF();
    },
    "get_team", [](WeakWrapper<ScriptedCharacter>& character) -> Team {
      return character.Unwrap()->GetTeam();
    },
    "is_team", [](WeakWrapper<ScriptedCharacter>& character, Team team) -> bool {
      return character.Unwrap()->Teammate(team);
    },
    "erase", [](WeakWrapper<ScriptedCharacter>& character) {
      character.Unwrap()->Erase();
    },
    "delete", [](WeakWrapper<ScriptedCharacter>& character) {
      character.Unwrap()->Delete();
    },
    "get_texture", [](WeakWrapper<ScriptedCharacter>& character) -> std::shared_ptr<Texture> {
      return character.Unwrap()->getTexture();
    },
    "set_texture", [](WeakWrapper<ScriptedCharacter>& character, std::shared_ptr<Texture> texture) {
      character.Unwrap()->setTexture(texture);
    },
    "set_shadow", sol::overload(
      [](WeakWrapper<ScriptedCharacter>& character, Entity::Shadow type) {
        character.Unwrap()->SetShadowSprite(type);
      },
      [](WeakWrapper<ScriptedCharacter>& character, std::shared_ptr<sf::Texture> shadow) {
        character.Unwrap()->SetShadowSprite(shadow);
      }
    ),
    "show_shadow", [](WeakWrapper<ScriptedCharacter>& character, bool enabled) {
        character.Unwrap()->ShowShadow(enabled);
    },
    "create_node", [](WeakWrapper<ScriptedCharacter>& character) -> WeakWrapper<SpriteProxyNode> {
      auto child = std::make_shared<SpriteProxyNode>();
      character.Unwrap()->AddNode(child);

      return WeakWrapper(child);
    },
    "get_name", [](WeakWrapper<ScriptedCharacter>& character) -> std::string {
      return character.Unwrap()->GetName();
    },
    "set_name", [](WeakWrapper<ScriptedCharacter>& character, std::string name) {
      character.Unwrap()->SetName(name);
    },
    "get_health", [](WeakWrapper<ScriptedCharacter>& character) -> int {
      return character.Unwrap()->GetHealth();
    },
    "get_max_health", [](WeakWrapper<ScriptedCharacter>& character) -> int {
      return character.Unwrap()->GetMaxHealth();
    },
    "set_health", [](WeakWrapper<ScriptedCharacter>& character, int health) {
      character.Unwrap()->SetHealth(health);
    },
    "get_rank", [](WeakWrapper<ScriptedCharacter>& character) -> Character::Rank {
      return character.Unwrap()->GetRank();
    },
    "toggle_hitbox", [](WeakWrapper<ScriptedCharacter>& character, bool enabled) {
      return character.Unwrap()->EnableHitbox(enabled);
    },
    "share_tile", [](WeakWrapper<ScriptedCharacter>& character, bool share) {
      character.Unwrap()->ShareTileSpace(share);
    },
    "register_component", [](WeakWrapper<ScriptedCharacter>& character, WeakWrapper<ScriptedComponent>& component) {
      character.Unwrap()->RegisterComponent(component.Release());
    },
    "add_defense_rule", [](WeakWrapper<ScriptedCharacter>& character, DefenseRule* defenseRule) {
      character.Unwrap()->AddDefenseRule(defenseRule->shared_from_this());
    },
    "remove_defense_rule", [](WeakWrapper<ScriptedCharacter>& character, DefenseRule* defenseRule) {
      character.Unwrap()->RemoveDefenseRule(defenseRule);
    },
    "get_offset", [](WeakWrapper<ScriptedCharacter>& character) -> sf::Vector2f {
      return character.Unwrap()->GetDrawOffset();
    },
    "set_offset", [](WeakWrapper<ScriptedCharacter>& character, float x, float y) {
      character.Unwrap()->SetDrawOffset(x, y);
    },
    "set_height", [](WeakWrapper<ScriptedCharacter>& character, float height) {
      character.Unwrap()->SetHeight(height);
    },
    "get_height", [](WeakWrapper<ScriptedCharacter>& character) -> float {
      return character.Unwrap()->GetHeight();
    },
    "set_elevation", [](WeakWrapper<ScriptedCharacter>& character, float elevation) {
      character.Unwrap()->SetElevation(elevation);
    },
    "get_elevation", [](WeakWrapper<ScriptedCharacter>& character) -> float {
      return character.Unwrap()->GetElevation();
    },
    "get_animation", [](WeakWrapper<ScriptedCharacter>& character) -> AnimationWrapper {
      auto& animation = character.Unwrap()->GetAnimationObject();
      return AnimationWrapper(character.GetWeak(), animation);
    },
    "shake_camera", [](WeakWrapper<ScriptedCharacter>& character, double power, float duration) {
      character.Unwrap()->ShakeCamera(power, duration);
    },
    "toggle_counter", [](WeakWrapper<ScriptedCharacter>& character, bool on) {
      character.Unwrap()->ToggleCounter(on);
    },
    "register_status_callback", [](WeakWrapper<ScriptedCharacter>& character, const Hit::Flags& flag, sol::stack_object callbackObject) {
      sol::protected_function callback = callbackObject;
      character.Unwrap()->RegisterStatusCallback(flag, [callback] {
        auto result = callback();

        if (!result.valid()) {
          sol::error error = result;
          Logger::Log(error.what());
        }
      });
    },
    "set_explosion_behavior", [](WeakWrapper<ScriptedCharacter>& character, int num, double speed, bool isBoss) {
      character.Unwrap()->SetExplosionBehavior(num, speed, isBoss);
    },
    "update_func", sol::property(
      [](WeakWrapper<ScriptedCharacter>& character) { return character.Unwrap()->update_func; },
      [](WeakWrapper<ScriptedCharacter>& character, sol::stack_object value) {
        character.Unwrap()->update_func = VerifyLuaCallback(value);
      }
    ),
    "delete_func", sol::property(
      [](WeakWrapper<ScriptedCharacter>& character) { return character.Unwrap()->delete_func; },
      [](WeakWrapper<ScriptedCharacter>& character, sol::stack_object value) {
        character.Unwrap()->delete_func = VerifyLuaCallback(value);
      }
    ),
    "on_spawn_func", sol::property(
      [](WeakWrapper<ScriptedCharacter>& character) { return character.Unwrap()->on_spawn_func; },
      [](WeakWrapper<ScriptedCharacter>& character, sol::stack_object value) {
        character.Unwrap()->on_spawn_func = VerifyLuaCallback(value);
      }
    ),
    "battle_start_func", sol::property(
      [](WeakWrapper<ScriptedCharacter>& character) { return character.Unwrap()->battle_start_func; },
      [](WeakWrapper<ScriptedCharacter>& character, sol::stack_object value) {
        character.Unwrap()->battle_start_func = VerifyLuaCallback(value);
      }
    ),
    "battle_end_func", sol::property(
      [](WeakWrapper<ScriptedCharacter>& character) { return character.Unwrap()->battle_end_func; },
      [](WeakWrapper<ScriptedCharacter>& character, sol::stack_object value) {
        character.Unwrap()->battle_end_func = VerifyLuaCallback(value);
      }
    ),
    "can_move_to_func", sol::property(
      [](WeakWrapper<ScriptedCharacter>& character) { return character.Unwrap()->can_move_to_func; },
      [](WeakWrapper<ScriptedCharacter>& character, sol::stack_object value) {
        character.Unwrap()->can_move_to_func = VerifyLuaCallback(value);
      }
    ),
    "on_countered", sol::property(
      [](WeakWrapper<ScriptedCharacter>& character) { return character.Unwrap()->on_countered_func; },
      [](WeakWrapper<ScriptedCharacter>& character, sol::stack_object value) {
        character.Unwrap()->on_countered_func = VerifyLuaCallback(value);
      }
    )
  );
}
#endif
