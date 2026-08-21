<script lang="ts">
  import { InboxManager, type ConversationItem } from "../../state/inbox";
  import AudioPlayer from "../../components/AudioPlayer.svelte";
  import { translations, type Locale } from "@shifa/shared";

  let { currentLocale = "en" as Locale } = $props<{ currentLocale?: Locale }>();
  let t = $derived(translations[currentLocale]);

  // Sample data for console view
  const mockConversations: ConversationItem[] = [
    {
      id: "c-1",
      customerId: "cust-1",
      customerName: "Dr. Tariq Mahmood",
      phone: "+923001234567",
      branchId: "b-lhr",
      lastMessage: "I need 2 boxes of Augmentin 625mg delivered to Model Town.",
      hasPrescription: true,
      unreadCount: 1,
      status: "ACTIVE",
      messages: [
        {
          id: "m-1",
          sender: "CUSTOMER",
          text: "Salam, please send 2 boxes of Augmentin 625mg.",
          timestamp: "10:30 AM",
        },
        {
          id: "m-2",
          sender: "AI_DRAFT",
          text: "Walaikum Assalam Dr. Tariq. Augmentin 625mg is available at Rs 1,450 per box. Since this is an antibiotic (Rx), our pharmacist will review your order.",
          aiConfidence: 0.96,
          timestamp: "10:31 AM",
        },
      ],
    },
    {
      id: "c-2",
      customerId: "cust-2",
      customerName: "Ayesha Bibi",
      phone: "+923214567890",
      branchId: "b-lhr",
      lastMessage: "Voice note received",
      hasPrescription: false,
      unreadCount: 0,
      status: "ACTIVE",
      messages: [
        {
          id: "m-3",
          sender: "CUSTOMER",
          mediaType: "AUDIO",
          audioTranscript: "Mujhe Panadol Extra ka aik patta bhej dain sham tak.",
          timestamp: "11:15 AM",
        },
      ],
    },
  ];

  let manager = $state(new InboxManager(mockConversations));
  let draftEditText = $state("");
  let isEditingDraft = $state(false);

  function handleSend(msgId: string) {
    if (!manager.selectedConversationId) return;
    manager.sendAiDraft(manager.selectedConversationId, msgId);
  }

  function handleEditStart(text: string) {
    draftEditText = text;
    isEditingDraft = true;
  }

  function handleEditSave(msgId: string) {
    if (!manager.selectedConversationId) return;
    manager.editAiDraft(manager.selectedConversationId, msgId, draftEditText);
    isEditingDraft = false;
  }

  function handleDiscard(msgId: string) {
    if (!manager.selectedConversationId) return;
    manager.discardAiDraft(manager.selectedConversationId, msgId);
  }
</script>

<div class="flex h-[calc(100vh-50px)] bg-slate-50 text-slate-800">
  <!-- Pane 1: Conversations List -->
  <aside class="w-80 border-e border-slate-200 bg-white flex flex-col">
    <div class="p-3 border-b border-slate-200 flex items-center justify-between">
      <h2 class="font-bold text-sm text-slate-800">{t.inbox.title}</h2>
      <span class="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded-full font-mono">
        {manager.conversations.length}
      </span>
    </div>

    {#if manager.reconnecting}
      <div class="bg-amber-500 text-white text-xs px-3 py-1.5 font-medium flex items-center gap-2">
        <span class="animate-spin">🔄</span>
        <span>{t.common.reconnect}</span>
      </div>
    {/if}

    <div class="overflow-y-auto flex-1 divide-y divide-slate-100">
      {#each manager.conversations as conv}
        <button
          onclick={() => (manager.selectedConversationId = conv.id)}
          class="w-full text-start p-3 transition-colors flex flex-col gap-1 rounded-lg {manager.selectedConversationId === conv.id ? 'bg-teal-50/90 text-teal-950 ring-1 ring-teal-500/30 font-medium' : 'hover:bg-slate-50'}"
        >
          <div class="flex items-center justify-between">
            <span class="font-semibold text-sm text-slate-900">{conv.customerName}</span>
            {#if conv.hasPrescription}
              <span class="text-[10px] bg-purple-100 text-purple-700 px-1.5 py-0.5 rounded font-bold">Rx</span>
            {/if}
          </div>
          <p class="text-xs text-slate-500 truncate">{conv.lastMessage}</p>
        </button>
      {/each}
    </div>
  </aside>

  <!-- Pane 2: Message Thread -->
  <main class="flex-1 flex flex-col bg-slate-100">
    {#if manager.selectedConversation}
      <div class="p-3 bg-white border-b border-slate-200 flex items-center justify-between">
        <div>
          <h3 class="font-bold text-sm text-slate-900">{manager.selectedConversation.customerName}</h3>
          <span class="text-xs text-slate-500 font-mono">{manager.selectedConversation.phone}</span>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto p-4 flex flex-col gap-3">
        {#each manager.selectedConversation.messages as msg}
          {#if msg.sender === "CUSTOMER"}
            <div class="self-start max-w-md bg-white p-3 rounded-xl rounded-tl-none shadow-sm border border-slate-200">
              {#if msg.mediaType === "AUDIO"}
                <AudioPlayer transcript={msg.audioTranscript} />
              {:else}
                <p class="text-sm text-slate-800">{msg.text}</p>
              {/if}
              <span class="text-[10px] text-slate-400 mt-1 block">{msg.timestamp}</span>
            </div>
          {:else if msg.sender === "AI_DRAFT"}
            <div class="self-end max-w-lg bg-teal-50 p-3 rounded-xl border border-teal-200">
              <div class="flex items-center justify-between mb-2">
                <span class="text-[10px] uppercase font-bold text-teal-800 tracking-wider bg-teal-100 px-2 py-0.5 rounded">
                  {t.inbox.draftBadge} ({(msg.aiConfidence! * 100).toFixed(0)}%)
                </span>
              </div>

              {#if isEditingDraft}
                <textarea
                  bind:value={draftEditText}
                  class="w-full text-sm p-2 border border-teal-300 rounded bg-white focus:outline-none focus:ring-1 focus:ring-teal-500"
                  rows="3"
                ></textarea>
                <div class="flex justify-end gap-2 mt-2">
                  <button onclick={() => (isEditingDraft = false)} class="text-xs px-2 py-1 bg-slate-200 rounded">{t.common.cancel}</button>
                  <button onclick={() => handleEditSave(msg.id)} class="text-xs px-2 py-1 bg-teal-600 text-white rounded">{t.common.save}</button>
                </div>
              {:else}
                <p class="text-sm text-slate-800 mb-3">{msg.text}</p>
                <div class="flex items-center gap-2">
                  <button onclick={() => handleSend(msg.id)} class="text-xs px-3 py-1 bg-teal-600 text-white font-medium rounded hover:bg-teal-700">
                    {t.inbox.sendDraft}
                  </button>
                  <button onclick={() => handleEditStart(msg.text || '')} class="text-xs px-3 py-1 bg-white border border-slate-300 text-slate-700 font-medium rounded hover:bg-slate-50">
                    {t.inbox.editDraft}
                  </button>
                  <button onclick={() => handleDiscard(msg.id)} class="text-xs px-3 py-1 text-red-600 hover:bg-red-50 rounded">
                    {t.inbox.discardDraft}
                  </button>
                </div>
              {/if}
            </div>
          {:else}
            <div class="self-end max-w-md bg-teal-600 text-white p-3 rounded-xl rounded-tr-none shadow-sm">
              <p class="text-sm">{msg.text}</p>
              <span class="text-[10px] text-teal-100 mt-1 block">{msg.timestamp}</span>
            </div>
          {/if}
        {/each}
      </div>
    {:else}
      <div class="flex-1 flex items-center justify-center text-slate-400 text-sm">
        {t.common.empty}
      </div>
    {/if}
  </main>

  <!-- Pane 3: Context Sidebar -->
  <aside class="w-72 border-s border-slate-200 bg-white p-4">
    <h4 class="font-bold text-xs uppercase text-slate-400 tracking-wider mb-3">Customer Context</h4>
    {#if manager.selectedConversation}
      <div class="flex flex-col gap-2 text-xs">
        <div><span class="text-slate-500">Name:</span> <span class="font-semibold">{manager.selectedConversation.customerName}</span></div>
        <div><span class="text-slate-500">Phone:</span> <span class="font-mono">{manager.selectedConversation.phone}</span></div>
        <div><span class="text-slate-500">Prescription:</span> <span class="font-semibold">{manager.selectedConversation.hasPrescription ? 'Yes' : 'No'}</span></div>
      </div>
    {/if}
  </aside>
</div>
