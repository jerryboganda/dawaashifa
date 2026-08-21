<script lang="ts">
  import { onMount } from "svelte";
  import { apiFetch } from "../../lib/api";
  import { InboxManager, type ConversationItem, type MessageItem } from "../../state/inbox";
  import AudioPlayer from "../../components/AudioPlayer.svelte";
  import { translations, type Locale, type ConversationDto } from "@shifa/shared";

  let { currentLocale = "en" as Locale } = $props<{ currentLocale?: Locale }>();
  let t = $derived(translations[currentLocale]);

  let manager = $state(new InboxManager([]));
  let draftEditText = $state("");
  let isEditingDraft = $state(false);
  let newMessageText = $state("");
  let isLoading = $state(true);
  let errorMessage = $state<string | null>(null);

  async function loadConversations() {
    isLoading = true;
    errorMessage = null;
    try {
      const convs = await apiFetch<ConversationDto[]>("/api/v1/conversations");
      const mapped: ConversationItem[] = convs.map((c) => ({
        id: c.id,
        customerId: c.customer_id,
        customerName: c.customer_name || "Customer",
        phone: c.customer_phone || "+923000000000",
        branchId: c.branch_id || "b-default",
        lastMessage: c.last_message_preview || "No messages yet",
        hasPrescription: c.has_prescription,
        unreadCount: c.unread_count,
        status: (c.status as any) || "ACTIVE",
        messages: (c.messages || []).map((m: any) => ({
          id: m.id,
          sender: m.sender_type === "AI_DRAFT" ? "AI_DRAFT" : m.direction === "INBOUND" ? "CUSTOMER" : "AGENT",
          text: m.body,
          mediaUrl: m.media_object_key,
          mediaType: m.content_type === "audio" ? "AUDIO" : m.content_type === "image" ? "IMAGE" : undefined,
          audioTranscript: m.audio_transcript,
          aiConfidence: m.ai_confidence,
          timestamp: m.created_at ? new Date(m.created_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "Just now",
        })),
      }));

      manager = new InboxManager(mapped);
    } catch (e: any) {
      errorMessage = e.message || "Failed to load WhatsApp conversations";
    } finally {
      isLoading = false;
    }
  }

  async function handleSend(msgId: string) {
    if (!manager.selectedConversationId) return;
    try {
      await apiFetch(`/api/v1/messages/${msgId}`, {
        method: "PATCH",
        body: JSON.stringify({ action: "APPROVE_AI_DRAFT" }),
      });
      manager.sendAiDraft(manager.selectedConversationId, msgId);
    } catch (e: any) {
      errorMessage = e.message || "Failed to send message";
    }
  }

  function handleEditStart(text: string) {
    draftEditText = text;
    isEditingDraft = true;
  }

  async function handleEditSave(msgId: string) {
    if (!manager.selectedConversationId) return;
    try {
      await apiFetch(`/api/v1/messages/${msgId}`, {
        method: "PATCH",
        body: JSON.stringify({
          action: "OVERRIDE_AI_DRAFT",
          new_body: draftEditText,
        }),
      });
      manager.editAiDraft(manager.selectedConversationId, msgId, draftEditText);
      isEditingDraft = false;
    } catch (e: any) {
      errorMessage = e.message || "Failed to save edited AI draft";
    }
  }

  async function handleDiscard(msgId: string) {
    if (!manager.selectedConversationId) return;
    try {
      await apiFetch(`/api/v1/messages/${msgId}`, {
        method: "PATCH",
        body: JSON.stringify({ action: "DISCARD_AI_DRAFT" }),
      });
      manager.discardAiDraft(manager.selectedConversationId, msgId);
    } catch (e: any) {
      errorMessage = e.message || "Failed to discard AI draft";
    }
  }

  async function handleSendManualMessage() {
    if (!manager.selectedConversationId || !newMessageText.trim()) return;
    try {
      await apiFetch(`/api/v1/conversations/${manager.selectedConversationId}/messages`, {
        method: "POST",
        body: JSON.stringify({
          body: newMessageText.trim(),
          content_type: "text",
        }),
      });

      const conv = manager.selectedConversation;
      if (conv) {
        conv.messages.push({
          id: `msg-${Date.now()}`,
          sender: "AGENT",
          text: newMessageText.trim(),
          timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        });
      }
      newMessageText = "";
    } catch (e: any) {
      errorMessage = e.message || "Failed to send WhatsApp message";
    }
  }

  onMount(() => {
    loadConversations();
  });
</script>

<div class="flex h-[calc(100vh-50px)] bg-slate-50 text-slate-800">
  <!-- Pane 1: Conversations List -->
  <aside class="w-80 border-e border-slate-200 bg-white flex flex-col">
    <div class="p-3 border-b border-slate-200 flex items-center justify-between">
      <h2 class="font-bold text-sm text-slate-800">{t.inbox.title}</h2>
      <div class="flex items-center gap-2">
        <button onclick={loadConversations} class="text-xs text-slate-500 hover:text-slate-800">🔄</button>
        <span class="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded-full font-mono">
          {manager.conversations.length}
        </span>
      </div>
    </div>

    {#if manager.reconnecting}
      <div class="bg-amber-500 text-white text-xs px-3 py-1.5 font-medium flex items-center gap-2">
        <span class="animate-spin">🔄</span>
        <span>{t.common.reconnect}</span>
      </div>
    {/if}

    <div class="flex-1 overflow-y-auto divide-y divide-slate-100">
      {#if isLoading}
        <div class="p-4 text-center text-xs text-slate-400">Loading inbox...</div>
      {:else if manager.conversations.length === 0}
        <div class="p-4 text-center text-xs text-slate-400">No active conversations.</div>
      {:else}
        {#each manager.conversations as conv}
          <button
            onclick={() => (manager.selectedConversationId = conv.id)}
            class="w-full text-start p-3 hover:bg-slate-50 transition-colors flex flex-col gap-1 {manager.selectedConversationId === conv.id ? 'bg-teal-50/50 border-s-4 border-teal-600' : ''}"
          >
            <div class="flex items-center justify-between">
              <span class="font-bold text-xs text-slate-900">{conv.customerName}</span>
              {#if conv.hasPrescription}
                <span class="text-[10px] bg-purple-100 text-purple-700 px-1 rounded font-semibold">Rx</span>
              {/if}
            </div>
            <span class="text-[11px] text-slate-500 truncate">{conv.phone}</span>
            <p class="text-xs text-slate-600 truncate mt-1">{conv.lastMessage}</p>
          </button>
        {/each}
      {/if}
    </div>
  </aside>

  <!-- Pane 2: Message Thread & AI Draft Area -->
  <main class="flex-1 flex flex-col bg-slate-100">
    {#if errorMessage}
      <div class="bg-red-600 text-white text-xs px-4 py-2 font-bold flex items-center justify-between">
        <span>⚠️ {errorMessage}</span>
        <button onclick={() => (errorMessage = null)} class="text-white hover:text-red-200">✕</button>
      </div>
    {/if}

    {#if manager.selectedConversation}
      <header class="bg-white border-b border-slate-200 px-4 py-2.5 flex items-center justify-between">
        <div>
          <h3 class="font-bold text-sm text-slate-900">{manager.selectedConversation.customerName}</h3>
          <span class="text-xs text-slate-500 font-mono">{manager.selectedConversation.phone}</span>
        </div>
      </header>

      <div class="flex-1 p-4 overflow-y-auto flex flex-col gap-3">
        {#each manager.selectedConversation.messages as msg}
          {#if msg.sender === "CUSTOMER"}
            <div class="flex flex-col items-start max-w-[70%]">
              <div class="bg-white p-3 rounded-2xl rounded-tl-sm shadow-sm border border-slate-200 text-xs">
                {#if msg.mediaType === "AUDIO" && msg.audioTranscript}
                  <div class="mb-2">
                    <AudioPlayer audioUrl={msg.mediaUrl || ""} durationSec={12} transcript={msg.audioTranscript} />
                  </div>
                {:else}
                  <p>{msg.text}</p>
                {/if}
                <span class="text-[10px] text-slate-400 mt-1 block">{msg.timestamp}</span>
              </div>
            </div>
          {:else if msg.sender === "AGENT"}
            <div class="flex flex-col items-end self-end max-w-[70%]">
              <div class="bg-teal-700 text-white p-3 rounded-2xl rounded-tr-sm shadow-sm text-xs">
                <p>{msg.text}</p>
                <span class="text-[10px] text-teal-200 mt-1 block">{msg.timestamp}</span>
              </div>
            </div>
          {:else if msg.sender === "AI_DRAFT"}
            <div class="w-full bg-amber-50 border border-amber-200 rounded-xl p-3 flex flex-col gap-2 my-2">
              <div class="flex items-center justify-between text-xs">
                <span class="font-bold text-amber-900 flex items-center gap-1.5">
                  <span>🤖</span> {t.inbox.aiDrafted} ({Math.round((msg.aiConfidence || 0.9) * 100)}% confidence)
                </span>
              </div>

              {#if isEditingDraft}
                <textarea
                  bind:value={draftEditText}
                  class="w-full text-xs p-2 border border-amber-300 rounded bg-white focus:outline-none focus:ring-1 focus:ring-amber-500"
                  rows="3"
                ></textarea>
                <div class="flex gap-2">
                  <button onclick={() => handleEditSave(msg.id)} class="px-3 py-1 bg-amber-600 text-white text-xs font-bold rounded">
                    Save
                  </button>
                  <button onclick={() => (isEditingDraft = false)} class="px-3 py-1 bg-slate-200 text-slate-700 text-xs rounded">
                    Cancel
                  </button>
                </div>
              {:else}
                <p class="text-xs text-amber-950">{msg.text}</p>
                <div class="flex gap-2 pt-1 border-t border-amber-200">
                  <button onclick={() => handleSend(msg.id)} class="px-3 py-1 bg-teal-600 text-white text-xs font-bold rounded hover:bg-teal-700">
                    {t.inbox.sendDraft}
                  </button>
                  <button onclick={() => handleEditStart(msg.text || "")} class="px-3 py-1 bg-amber-200 text-amber-900 text-xs font-semibold rounded hover:bg-amber-300">
                    {t.inbox.editDraft}
                  </button>
                  <button onclick={() => handleDiscard(msg.id)} class="px-3 py-1 bg-slate-200 text-slate-700 text-xs rounded hover:bg-slate-300">
                    {t.inbox.discardDraft}
                  </button>
                </div>
              {/if}
            </div>
          {/if}
        {/each}
      </div>

      <!-- Outbound Composer -->
      <footer class="p-3 bg-white border-t border-slate-200 flex gap-2">
        <input
          type="text"
          bind:value={newMessageText}
          placeholder="Type a WhatsApp reply..."
          onkeydown={(e) => e.key === "Enter" && handleSendManualMessage()}
          class="flex-1 text-xs p-2 border border-slate-300 rounded focus:outline-none focus:ring-1 focus:ring-teal-500"
        />
        <button
          onclick={handleSendManualMessage}
          class="px-4 py-2 bg-teal-600 text-white text-xs font-bold rounded hover:bg-teal-700"
        >
          Send
        </button>
      </footer>
    {:else}
      <div class="flex-1 flex items-center justify-center text-slate-400 text-xs">
        Select a conversation from the sidebar to view thread.
      </div>
    {/if}
  </main>
</div>
