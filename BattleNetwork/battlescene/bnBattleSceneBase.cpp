#include "bnBattleSceneBase.h"

#include <assert.h>
#include <Segues/WhiteWashFade.h>
#include <Segues/BlackWashFade.h>
#include <Segues/PixelateBlackWashFade.h>

#include "../bnTextureResourceManager.h"
#include "../bnShaderResourceManager.h"
#include "../bnInputManager.h"
#include "../bnMob.h"
#include "../bnCardAction.h"
#include "../bnPlayerHealthUI.h"
#include "../bnPlayerEmotionUI.h"
#include "../bnUndernetBackground.h"
#include "../bnWeatherBackground.h"
#include "../bnRobotBackground.h"
#include "../bnMedicalBackground.h"
#include "../bnACDCBackground.h"
#include "../bnMiscBackground.h"
#include "../bnSecretBackground.h"
#include "../bnJudgeTreeBackground.h"
#include "../bnLanBackground.h"
#include "../bnGraveyardBackground.h"
#include "../bnVirusBackground.h"

// Combos are counted if more than one enemy is hit within x frames
// The game is clocked to display 60 frames per second
// If x = 20 frames, then we want a combo hit threshold of 20/60 = 0.3 seconds
#define COMBO_HIT_THRESHOLD_SECONDS 20.0f/60.0f

using swoosh::types::segue;
using swoosh::Activity;
using swoosh::ActivityController;

BattleSceneBase::BattleSceneBase(ActivityController& controller, BattleSceneBaseProps& props, BattleResultsFunc onEnd) :
  Scene(controller),
  cardActionListener(this->getController().CardPackageManager()),
  player(props.player),
  programAdvance(props.programAdvance),
  comboDeleteCounter(0),
  totalCounterMoves(0),
  totalCounterDeletions(0),
  whiteShader(Shaders().GetShader(ShaderType::WHITE_FADE)),
  backdropShader(Shaders().GetShader(ShaderType::BLACK_FADE)),
  yellowShader(Shaders().GetShader(ShaderType::YELLOW)),
  heatShader(Shaders().GetShader(ShaderType::SPOT_DISTORTION)),
  iceShader(Shaders().GetShader(ShaderType::SPOT_REFLECTION)),
  // cap of 8 cards, 8 cards drawn per turn
  cardCustGUI(CardSelectionCust::Props{ std::move(props.folder), &getController().CardPackageManager(), 8, 8 }),
  mobFont(Font::Style::thick),
  camera(sf::View{ sf::Vector2f(240, 160), sf::Vector2f(480, 320) }),
  onEndCallback(onEnd),
  channel(this)
{
  /*
  Set Scene*/
  field = props.field;
  CharacterDeleteListener::Subscribe(*field);
  field->SetScene(this); // event emitters during battle needs the active scene

  player->ChangeState<PlayerIdleState>();
  player->ToggleTimeFreeze(false);
  field->AddEntity(player, 2, 2);

  /*
  Background for scene*/
  background = props.background;

  if (!background) {
    int randBG = rand() % 8;

    if (randBG == 0) {
      background = std::make_shared<LanBackground>();
    }
    else if (randBG == 1) {
      background = std::make_shared<GraveyardBackground>();
    }
    else if (randBG == 2) {
      background = std::make_shared<WeatherBackground>();
    }
    else if (randBG == 3) {
      background = std::make_shared<RobotBackground>();
    }
    else if (randBG == 4) {
      background = std::make_shared<MedicalBackground>();
    }
    else if (randBG == 5) {
      background = std::make_shared<ACDCBackground>();
    }
    else if (randBG == 6) {
      background = std::make_shared<MiscBackground>();
    }
    else if (randBG == 7) {
      background = std::make_shared<JudgeTreeBackground>();
    }
  }

  // Player UI
  cardUI = player->CreateComponent<PlayerSelectedCardsUI>(player, &getController().CardPackageManager());
  CardActionUseListener::Subscribe(*player);
  CardActionUseListener::Subscribe(*cardUI);

  auto healthUI = player->CreateComponent<PlayerHealthUI>(player);
  cardCustGUI.AddNode(healthUI);

  // Player Emotion
  this->emotionUI = player->CreateComponent<PlayerEmotionUI>(player);
  cardCustGUI.AddNode(emotionUI);

  emotionUI->Subscribe(cardCustGUI);

  /*
  Counter "reveal" ring
  */

  counterRevealAnim = Animation("resources/navis/counter_reveal.animation");
  counterRevealAnim << "DEFAULT" << Animator::Mode::Loop;

  counterReveal = std::make_shared<SpriteProxyNode>();
  counterReveal->setTexture(Textures().LoadFromFile(TexturePaths::MISC_COUNTER_REVEAL), true);
  counterReveal->EnableParentShader(false);
  counterReveal->SetLayer(-100);

  counterCombatRule = std::make_shared<CounterCombatRule>(this);

  // Load forms
  cardCustGUI.SetPlayerFormOptions(player->GetForms());

  // MOB UI
  mobBackdropSprite = sf::Sprite(*Textures().LoadFromFile(TexturePaths::MOB_NAME_BACKDROP));
  mobEdgeSprite = sf::Sprite(*Textures().LoadFromFile(TexturePaths::MOB_NAME_EDGE));

  mobBackdropSprite.setScale(2.f, 2.f);
  mobEdgeSprite.setScale(2.f, 2.f);

  // SHADERS
  // TODO: Load shaders if supported
  shaderCooldown = 0;

  if (whiteShader) {
    whiteShader->setUniform("texture", sf::Shader::CurrentTexture);
    whiteShader->setUniform("opacity", 0.5f);
    whiteShader->setUniform("texture", sf::Shader::CurrentTexture);
  }

  textureSize = getController().getVirtualWindowSize();

  if (iceShader) {
    iceShader->setUniform("texture", sf::Shader::CurrentTexture);
    iceShader->setUniform("sceneTexture", sf::Shader::CurrentTexture);
    iceShader->setUniform("textureSizeIn", sf::Glsl::Vec2((float)textureSize.x, (float)textureSize.y));
    iceShader->setUniform("shine", 0.2f);
  }

  isSceneInFocus = false;

  comboInfoTimer.start();
  multiDeleteTimer.start();

  HitListener::Subscribe(*player);

  setView(sf::Vector2u(480, 320));

  // add the camera to our event bus
  channel.Register(&camera);
}

BattleSceneBase::~BattleSceneBase() {
  // drop the camera from our event bus
  channel.Drop(&camera);

  for (auto&& elem : states) {
    delete elem;
  }

  for (auto&& elem : nodeToEdges) {
    delete elem.second;
  }

  for (auto& c : components) {
    c->scene = nullptr;
  }
}

const bool BattleSceneBase::DoubleDelete() const
{
  return didDoubleDelete;
}
const bool BattleSceneBase::TripleDelete() const
{
  return didTripleDelete;
}
;

const int BattleSceneBase::GetCounterCount() const {
  return totalCounterMoves;
}

const bool BattleSceneBase::IsSceneInFocus() const
{
  return isSceneInFocus;
}

const bool BattleSceneBase::IsQuitting() const
{
    return this->quitting;
}

void BattleSceneBase::OnCounter(Entity& victim, Entity& aggressor)
{
  Audio().Play(AudioType::COUNTER, AudioPriority::highest);

  if (&aggressor == player.get()) {
    totalCounterMoves++;

    if (victim.IsDeleted()) {
      totalCounterDeletions++;
    }

    didCounterHit = true;
    comboInfoTimer.reset();

    if (player->IsInForm() == false && player->GetEmotion() != Emotion::evil) {
      field->RevealCounterFrames(true);

      // node positions are relative to the parent node's origin
      auto bounds = player->getLocalBounds();
      counterReveal->setPosition(0, -bounds.height / 4.0f);
      player->AddNode(counterReveal);

      cardUI->SetMultiplier(2);

      player->SetEmotion(Emotion::full_synchro);

      // when players get hit by impact, battle scene takes back counter blessings
      player->AddDefenseRule(counterCombatRule);
    }
  }
}

void BattleSceneBase::OnDeleteEvent(Character& pending)
{
  // Track if player is being deleted
  if (!isPlayerDeleted && player.get() == &pending) {
    battleResults.runaway = false;
    isPlayerDeleted = true;
    player = nullptr;
  }

  auto pendingPtr = &pending;

  // Find any AI using this character as a target and free that pointer  
  field->FindEntities([pendingPtr](std::shared_ptr<Entity>& in) {
    auto agent = dynamic_cast<Agent*>(in.get());

    if (agent && agent->GetTarget().get() == pendingPtr) {
      agent->FreeTarget();
    }

    return false;
  });

  Logger::Logf("Removing %s from battle (ID: %d)", pending.GetName().c_str(), pending.GetID());
  mob->Forget(pending);

  if (mob->IsCleared()) {
    Audio().StopStream();
  }
}

const int BattleSceneBase::ComboDeleteSize() const 
{
  return comboInfoTimer.getElapsed().asSeconds() <= 1.0f ? comboDeleteCounter : 0;
}

const bool BattleSceneBase::Countered() const
{
  return comboInfoTimer.getElapsed().asSeconds() <= 1.0f && didCounterHit;
}

void BattleSceneBase::HighlightTiles(bool enable)
{
  this->highlightTiles = enable;
}

const double BattleSceneBase::GetCustomBarProgress() const
{
  return this->customProgress;
}

const double BattleSceneBase::GetCustomBarDuration() const
{
  return this->customDuration;
}

void BattleSceneBase::SetCustomBarProgress(double percentage)
{
  this->customProgress = this->customDuration * percentage;
}

void BattleSceneBase::SetCustomBarDuration(double maxTimeSeconds)
{
  this->customDuration = maxTimeSeconds;
}

void BattleSceneBase::SubscribeToCardActions(CardActionUsePublisher& publisher)
{
  cardActionListener.Subscribe(publisher);
  this->CardActionUseListener::Subscribe(publisher);
  cardUseSubscriptions.push_back(std::ref(publisher));
}

void BattleSceneBase::UnsubscribeFromCardActions(CardActionUsePublisher& publisher)
{
  // todo: cardListener.Unsubscribe(publisher);
}

const std::vector<std::reference_wrapper<CardActionUsePublisher>>& BattleSceneBase::GetCardActionSubscriptions() const
{
  return cardUseSubscriptions;
}

void BattleSceneBase::OnCardActionUsed(std::shared_ptr<CardAction> action, uint64_t timestamp)
{
  HandleCounterLoss(*action->GetActor(), true);
}

void BattleSceneBase::LoadMob(Mob& mob)
{
  this->mob = &mob;
}

void BattleSceneBase::HandleCounterLoss(Entity& subject, bool playsound)
{
  if (&subject == player.get()) {
    if (field->DoesRevealCounterFrames()) {
      player->RemoveNode(counterReveal);
      player->RemoveDefenseRule(counterCombatRule);
      player->SetEmotion(Emotion::normal);
      field->RevealCounterFrames(false);

      playsound ? Audio().Play(AudioType::COUNTER_BONUS, AudioPriority::highest) : 0;
    }
    cardUI->SetMultiplier(1);
  }
}

void BattleSceneBase::FilterSupportCards(std::vector<Battle::Card>& cards) {
  // Only remove the support cards in the queue. Increase the previous card damage by their support value.
  Battle::Card* card = nullptr;

  // Create a temp card list
  std::vector<Battle::Card> newCardList;

  for (int i = 0; i < cards.size(); i++) {
    if (cards[i].IsBooster()) {
      Logger::Logf("Booster card %s detected", cards[i].GetShortName().c_str());

      // check if we are tracking a non-booster card first
      if (card) {
        // booster cards do not modify other booster cards
        if (!card->IsBooster()) {
          int lastDamage = card->GetDamage();
          int buff = 0;

          // NOTE: hardcoded filter step for "Atk+X" cards
          if (cards[i].GetShortName().substr(0, 3) == "Atk") {
            std::string substr = cards[i].GetShortName().substr(4, cards[i].GetShortName().size() - 4).c_str();
            buff = atoi(substr.c_str());
          }

          card->ModDamage(buff);
        }

        continue; // skip the rest of the code below
      }
    }

    newCardList.push_back(cards[i]);
    card = &cards[i];
  }

  cards = std::move(newCardList);
}

#ifdef __ANDROID__
void BattleSceneBase::SetupTouchControls() {

}

void BattleSceneBase::ShutdownTouchControls() {

}
#endif

void BattleSceneBase::StartStateGraph(StateNode& start) {
  // kick-off and align all sprites on the screen
  field->Update(0);

  // set the current state ptr and kick-off
  this->current = &start.state;
  this->current->onStart();
}

void BattleSceneBase::onStart()
{
  isSceneInFocus = true;

  // Stream battle music
  if (mob && mob->HasCustomMusicPath()) {
    auto points = mob->GetLoopPoints();
    Audio().Stream(mob->GetCustomMusicPath(), true, points[0], points[1]);
  }
  else {
    if (mob == nullptr || !mob->IsBoss()) {
      Audio().Stream("resources/loops/loop_battle.ogg", true);
    }
    else {
      Audio().Stream("resources/loops/loop_boss_battle.ogg", true);
    }
  }
}

void BattleSceneBase::onUpdate(double elapsed) {
  this->elapsed = elapsed;

  camera.Update((float)elapsed);
  background->Update((float)elapsed);

  if (backdropShader) {
    backdropShader->setUniform("opacity", (float)backdropOpacity);
  }

  counterRevealAnim.Update((float)elapsed, counterReveal->getSprite());
  comboInfoTimer.update(sf::seconds(static_cast<float>(elapsed)));
  multiDeleteTimer.update(sf::seconds(static_cast<float>(elapsed)));
  battleTimer.update(sf::seconds(static_cast<float>(elapsed)));

  switch (backdropMode) {
  case backdrop::fadein:
    backdropOpacity = std::fmin(backdropMaxOpacity, backdropOpacity + (backdropFadeIncrements * elapsed));
    break;
  case backdrop::fadeout:
    backdropOpacity = std::fmax(0.0, backdropOpacity - (backdropFadeIncrements * elapsed));
    if (backdropOpacity == 0.0) {
      backdropAffectBG = false; // reset this effect
    }
    break;
  }

  // Register and eject any applicable components
  ProcessNewestComponents();

  if (!IsPlayerDeleted()) {
    cardUI->OnUpdate((float)elapsed);
  }

  cardCustGUI.Update((float)elapsed);

  newMobSize = mob? mob->GetMobCount() : 0;

  if (sf::Keyboard::isKeyPressed(sf::Keyboard::Escape)) {
    Quit(FadeOut::white);
    Audio().StopStream();
  }

  // State update
  if(!current) return;

  current->onUpdate(elapsed);

  auto componentsCopy = components;

  // Update components
  for (auto& c : componentsCopy) {
    if (c->Lifetime() == Component::lifetimes::ui && battleTimer.getElapsed().asMilliseconds() > 0) {
      c->Update((float)elapsed);
    }
    else if (c->Lifetime() == Component::lifetimes::battlestep) {
      // If the mob isn't cleared, only update when the battle-step timer is going
      // Otherwise, feel free to update as the battle is over (mob is cleared)
      bool updateBattleSteps = !mob->IsCleared() && !battleTimer.isPaused();
      updateBattleSteps = updateBattleSteps || mob->IsCleared();
      if (updateBattleSteps) {
        c->Update((float)elapsed);
      }
    }
  }

  // Track combo deletes
  if (lastMobSize != newMobSize && !isPlayerDeleted) {
    int counter = lastMobSize - newMobSize;
    if (multiDeleteTimer.getElapsed() <= sf::seconds(COMBO_HIT_THRESHOLD_SECONDS)) {
      comboDeleteCounter += counter;

      if (comboDeleteCounter == 2) {
        didDoubleDelete = true;
        comboInfoTimer.reset();
      }
      else if (comboDeleteCounter > 2) {
        didTripleDelete = true;
        comboInfoTimer.reset();
      }
    }
    else if (multiDeleteTimer.getElapsed() > sf::seconds(COMBO_HIT_THRESHOLD_SECONDS)) {
      comboDeleteCounter = counter;
    }
  }

  if (lastMobSize != newMobSize) {
    // prepare for another enemy deletion
    multiDeleteTimer.reset();
  }

  lastMobSize = newMobSize;

  for (auto iter = nodeToEdges.begin(); iter != nodeToEdges.end(); iter++) {
    if (iter->first == current) {
      if (iter->second->when()) {
        auto temp = iter->second->b;
        this->last = current;
        this->next = temp;
        current->onEnd(this->next);
        current = temp;
        current->onStart(this->last);
        current->onUpdate(0); // frame-perfect state visuals
        break;
      }
    }
  }

  if (player) {
    battleResults.playerHealth = player->GetHealth();
  }
}

void BattleSceneBase::onDraw(sf::RenderTexture& surface) {
  int tint = static_cast<int>((1.0f - backdropOpacity) * 255);

  if (!backdropAffectBG) {
    tint = 255;
  }

  background->SetOpacity(1.0f- backdropOpacity);

  surface.draw(*background);

  auto uis = std::vector<std::shared_ptr<UIComponent>>();

  auto allTiles = field->FindTiles([](Battle::Tile* tile) { return true; });
  auto viewOffset = getController().CameraViewOffset(camera);

  for (Battle::Tile* tile : allTiles) {
    if (tile->IsEdgeTile()) continue;

    bool yellowBlock = false;

    if (tile->IsHighlighted() && !this->IsCleared()) {
      if (!yellowShader) {
        yellowBlock = true;
      }
      else {
        tile->SetShader(yellowShader);
      }
    }
    else {
      tile->RevokeShader();
      tile->setColor(sf::Color::White);
    }

    tile->move(viewOffset);
    tile->setColor(sf::Color(tint, tint, tint, 255));
    surface.draw(*tile);
    tile->setColor(sf::Color::White);

    if (yellowBlock) {
      sf::RectangleShape block;
      block.setSize({40, 30});
      block.setScale(2.f, 2.f);
      block.setOrigin(20, 15);
      block.setFillColor(sf::Color::Yellow);
      block.setPosition(tile->getPosition());
      surface.draw(block);
    }

    tile->move(-viewOffset);
  }

  std::vector<Entity*> allEntities;

  for (Battle::Tile* tile : allTiles) {
    std::vector<Entity*> tileEntities;
    tile->FindEntities([&tileEntities, &allEntities](std::shared_ptr<Entity>& ent) {
      tileEntities.push_back(ent.get());
      allEntities.push_back(ent.get());
      return false;
    });

    std::sort(tileEntities.begin(), tileEntities.end(), [](Entity* A, Entity* B) { return A->GetLayer() > B->GetLayer(); });

    for (Entity* node : tileEntities) {
      sf::Vector2f offset = viewOffset + sf::Vector2f(0, -node->GetElevation());
      node->move(offset);
      surface.draw(*node);
      node->move(-offset);
    }
  }

  std::vector<Character*> allCharacters;

  // draw ui on top
  for (auto* ent : allEntities) {
    auto uis = ent->GetComponentsDerivedFrom<UIComponent>();

    for (auto& ui : uis) {
      if (ui->DrawOnUIPass()) {
        ui->move(viewOffset);
        surface.draw(*ui);
        ui->move(-viewOffset);
      }
    }

    // collect characters while drawing ui
    if (auto character = dynamic_cast<Character*>(ent)) {
      allCharacters.push_back(character);
    }
  }

  // draw extra card action graphics
  for (auto* c : allCharacters) {
    auto actionList = c->AsyncActionList();
    auto currAction = c->CurrentCardAction();

    for (auto& action : actionList) {
      surface.draw(*action);
    }

    if (currAction) {
      surface.draw(*currAction);
    }
  }

  // Draw whatever extra state stuff we want to have
  if (current) current->onDraw(surface);
}

void BattleSceneBase::onEnd()
{
  if (this->onEndCallback) {
    this->onEndCallback(battleResults);
  }
}

bool BattleSceneBase::IsPlayerDeleted() const
{
  return isPlayerDeleted;
}

std::shared_ptr<Player> BattleSceneBase::GetPlayer()
{
  return player;
}

std::shared_ptr<Field> BattleSceneBase::GetField()
{
  return field;
}

const std::shared_ptr<Field> BattleSceneBase::GetField() const
{
  return field;
}

CardSelectionCust& BattleSceneBase::GetCardSelectWidget()
{
  return cardCustGUI;
}

PlayerSelectedCardsUI& BattleSceneBase::GetSelectedCardsUI() {
  return *cardUI;
}

PlayerEmotionUI& BattleSceneBase::GetEmotionWindow()
{
  return *emotionUI;
}

Camera& BattleSceneBase::GetCamera()
{
  return camera;
}

PA& BattleSceneBase::GetPA()
{
  return programAdvance;
}

BattleResults& BattleSceneBase::BattleResultsObj()
{
  return battleResults;
}

const int BattleSceneBase::GetTurnCount()
{
  return turn;
}

const int BattleSceneBase::GetRoundCount()
{
  return round;
}

void BattleSceneBase::StartBattleStepTimer()
{
  battleTimer.start();
}

void BattleSceneBase::StopBattleStepTimer()
{
  battleTimer.pause();
}

void BattleSceneBase::BroadcastBattleStart()
{
  field->RequestBattleStart();
}

void BattleSceneBase::BroadcastBattleStop()
{
  field->RequestBattleStop();
}

void BattleSceneBase::IncrementTurnCount()
{
  turn++;
}

void BattleSceneBase::IncrementRoundCount()
{
  round++;
}

const sf::Time BattleSceneBase::GetElapsedBattleTime() {
  return battleTimer.getElapsed();
}

const bool BattleSceneBase::FadeInBackdrop(double amount, double to, bool affectBackground)
{
  backdropMode = backdrop::fadein;
  backdropFadeIncrements = amount;
  backdropMaxOpacity = to;
  backdropAffectBG = affectBackground;

  return (backdropOpacity >= to);
}

const bool BattleSceneBase::FadeOutBackdrop(double amount)
{
  backdropMode = backdrop::fadeout;
  backdropFadeIncrements = amount;

  return (backdropOpacity == 0.0);
}

std::vector<std::reference_wrapper<const Character>> BattleSceneBase::MobList()
{
  std::vector<std::reference_wrapper<const Character>> mobList;

  if (mob) {
    for (int i = 0; i < mob->GetMobCount(); i++) {
      mobList.push_back(mob->GetMobAt(i));
    }
  }

  return mobList;
}

void BattleSceneBase::Quit(const FadeOut& mode) {
  if(quitting) return; 

  // end the current state
  if(current) {
    current->onEnd();
    current = nullptr;
  }

  // NOTE: swoosh quirk
  if (getController().getStackSize() == 1) {
    getController().pop();
    quitting = true;
    return;
  }

  // Depending on the mode, use Swoosh's 
  // activity controller to fadeout with the right
  // visual appearance
  if(mode == FadeOut::white) {
    getController().pop<segue<WhiteWashFade>>();
  } else if(mode == FadeOut::black) {
    getController().pop<segue<BlackWashFade>>();
  }
  else {
    // mode == FadeOut::pixelate
    getController().pop<segue<PixelateBlackWashFade>>();
  }

  quitting = true;
}


// what to do if we inject a UIComponent, add it to the update and topmost scenenode stack
void BattleSceneBase::Inject(std::shared_ptr<MobHealthUI> other)
{
  other->scene = this;
  components.push_back(other);
  scenenodes.push_back(other);
}

void BattleSceneBase::Inject(std::shared_ptr<SelectedCardsUI> cardUI)
{
  this->SubscribeToCardActions(*cardUI);
}

// Default case: no special injection found for the type, just add it to our update loop
void BattleSceneBase::Inject(std::shared_ptr<Component> other)
{
  assert(other && "Component injected was nullptr");

  auto node = std::dynamic_pointer_cast<SceneNode>(other);
  if (node) { scenenodes.push_back(node); }

  other->scene = this;
  components.push_back(other);
}

void BattleSceneBase::Eject(Component::ID_t ID)
{
  auto iter = std::find_if(components.begin(), components.end(), 
    [ID](auto& in) { return in->GetID() == ID; }
  );

  if (iter != components.end()) {
    auto& component = **iter;

    auto node = dynamic_cast<SceneNode*>(&component);
    // TODO: dynamic casting could be entirely avoided by hashing IDs
    auto iter2 = std::find_if(scenenodes.begin(), scenenodes.end(), 
      [node](auto& in) { 
        return in.get() == node;
      }
    );

    if (iter2 != scenenodes.end()) {
      scenenodes.erase(iter2);
    }

    component.scene = nullptr;
    components.erase(iter);
  }
}

const bool BattleSceneBase::IsCleared()
{
  return mob? mob->IsCleared() : true;
}

void BattleSceneBase::Link(StateNode& a, StateNode& b, ChangeCondition when) {

  nodeToEdges.insert(std::make_pair(&(a.state), new Edge(&(a.state), &(b.state), when)));
}

void BattleSceneBase::ProcessNewestComponents()
{
  // effectively returns all of them
  std::vector<Entity*> entities;
  field->FindEntities([&entities](std::shared_ptr<Entity>& e) {
    entities.push_back(e.get());
    return false;
  });

  for (auto e : entities) {
    if (e->components.size() > 0) {
      // update the ledger
      // this step proceeds the lastComponentID update
      auto latestID = e->components[0]->GetID();

      if (e->lastComponentID < latestID) {
        //std::cout << "latestID: " << latestID << " lastComponentID: " << e->lastComponentID << "\n";

        for (auto& c : e->components) {
          if (!c) continue;

          // Older components are last in order, we're done
          if (c->GetID() <= e->lastComponentID) break;

          // Local components are not a part of the battle scene and do not get injected
          if (c->Lifetime() != Component::lifetimes::local) {
            c->Inject(*this);
          }
        }

        e->lastComponentID = latestID;
      }
    }
  }

#ifdef __ANDROID__
  SetupTouchControls();
#endif
}

void BattleSceneBase::onLeave() {
#ifdef __ANDROID__
  ShutdownTouchControls();
#endif
}

#ifdef __ANDROID__
void BattleSceneBase::SetupTouchControls() {
  /* Android touch areas*/
  TouchArea& rightSide = TouchArea::create(sf::IntRect(240, 0, 240, 320));

  rightSide.enableExtendedRelease(true);
  releasedB = false;

  rightSide.onTouch([]() {
    INPUTx.VirtualKeyEvent(InputEvent::RELEASED_A);
    });

  rightSide.onRelease([this](sf::Vector2i delta) {
    if (!releasedB) {
      INPUTx.VirtualKeyEvent(InputEvent::PRESSED_A);
    }

    releasedB = false;

    });

  rightSide.onDrag([this](sf::Vector2i delta) {
    if (delta.x < -25 && !releasedB) {
      INPUTx.VirtualKeyEvent(InputEvent::PRESSED_B);
      INPUTx.VirtualKeyEvent(InputEvent::RELEASED_B);
      releasedB = true;
    }
    });

  rightSide.onDefault([this]() {
    releasedB = false;
    });

  TouchArea& custSelectButton = TouchArea::create(sf::IntRect(100, 0, 380, 100));
  custSelectButton.onTouch([]() {
    INPUTx.VirtualKeyEvent(InputEvent::PRESSED_START);
    });
  custSelectButton.onRelease([](sf::Vector2i delta) {
    INPUTx.VirtualKeyEvent(InputEvent::RELEASED_START);
    });

  TouchArea& dpad = TouchArea::create(sf::IntRect(0, 0, 240, 320));
  dpad.enableExtendedRelease(true);
  dpad.onDrag([](sf::Vector2i delta) {
    Logger::Log("dpad delta: " + std::to_string(delta.x) + ", " + std::to_string(delta.y));

    if (delta.x > 30) {
      INPUTx.VirtualKeyEvent(InputEvent::PRESSED_RIGHT);
    }

    if (delta.x < -30) {
      INPUTx.VirtualKeyEvent(InputEvent::PRESSED_LEFT);
    }

    if (delta.y > 30) {
      INPUTx.VirtualKeyEvent(InputEvent::PRESSED_DOWN);
    }

    if (delta.y < -30) {
      INPUTx.VirtualKeyEvent(InputEvent::PRESSED_UP);
    }
    });

  dpad.onRelease([](sf::Vector2i delta) {
    if (delta.x < -30) {
      INPUTx.VirtualKeyEvent(InputEvent::RELEASED_LEFT);
    }
    });
}

void BattleSceneBase::ShutdownTouchControls() {
  TouchArea::free();
}

#endif