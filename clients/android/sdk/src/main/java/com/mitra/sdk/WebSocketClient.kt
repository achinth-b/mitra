package com.mitra.sdk

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.flow
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.*
import okio.ByteString
import java.util.concurrent.TimeUnit

/**
 * Kotlin WebSocket Client for Mitra Prediction Market
 * 
 * Provides coroutine-based API for real-time updates
 */

@Serializable
sealed class WsMessage {
    @Serializable
    data class PriceUpdate(
        val event_id: String,
        val prices: Map<String, Double>,
        val timestamp: Long
    ) : WsMessage()

    @Serializable
    data class BetExecuted(
        val bet_id: String,
        val user: String,
        val outcome: String,
        val shares: Double,
        val price: Double
    ) : WsMessage()

    @Serializable
    data class EventSettled(
        val event_id: String,
        val winning_outcome: String
    ) : WsMessage()

    @Serializable
    data class Error(val message: String) : WsMessage()
}

typealias SubscriptionChannel = String // "event:{id}", "group:{id}", "user:{wallet}"

/**
 * WebSocket client for Mitra platform
 */
class MitraWebSocketClient(
    private val url: String,
    private val scope: CoroutineScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
) {
    private val client = OkHttpClient.Builder()
        .pingInterval(30, TimeUnit.SECONDS)
        .build()

    private var webSocket: WebSocket? = null
    private val subscriptions = mutableSetOf<SubscriptionChannel>()
    private val messageHandlers = mutableMapOf<String, MutableList<(WsMessage) -> Unit>>()
    
    private val _isConnected = MutableStateFlow(false)
    val isConnected: StateFlow<Boolean> = _isConnected

    private val json = Json { ignoreUnknownKeys = true }

    /**
     * Connect to WebSocket server
     */
    suspend fun connect() {
        withContext(Dispatchers.IO) {
            val request = Request.Builder()
                .url(url)
                .build()

            webSocket = client.newWebSocket(request, object : WebSocketListener() {
                override fun onOpen(webSocket: WebSocket, response: Response) {
                    _isConnected.value = true
                    
                    // Resubscribe to all channels
                    subscriptions.forEach { channel ->
                        subscribe(channel)
                    }
                }

                override fun onMessage(webSocket: WebSocket, text: String) {
                    scope.launch {
                        handleMessage(text)
                    }
                }

                override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
                    scope.launch {
                        handleMessage(bytes.utf8())
                    }
                }

                override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                    webSocket.close(1000, null)
                    _isConnected.value = false
                }

                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                    _isConnected.value = false
                }

                override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                    _isConnected.value = false
                    // TODO: Implement reconnection logic
                }
            })
        }
    }

    /**
     * Disconnect from WebSocket server
     */
    fun disconnect() {
        webSocket?.close(1000, "Client disconnect")
        webSocket = null
        subscriptions.clear()
    }

    /**
     * Subscribe to a channel
     */
    fun subscribe(channel: SubscriptionChannel) {
        subscriptions.add(channel)
        sendMessage(
            mapOf(
                "type" to "subscribe",
                "channel" to channel
            )
        )
    }

    /**
     * Unsubscribe from a channel
     */
    fun unsubscribe(channel: SubscriptionChannel) {
        subscriptions.remove(channel)
        sendMessage(
            mapOf(
                "type" to "unsubscribe",
                "channel" to channel
            )
        )
    }

    /**
     * Subscribe to event updates
     */
    fun subscribeToEvent(eventId: String) {
        subscribe("event:$eventId")
    }

    /**
     * Subscribe to group updates
     */
    fun subscribeToGroup(groupId: String) {
        subscribe("group:$groupId")
    }

    /**
     * Subscribe to user updates
     */
    fun subscribeToUser(walletAddress: String) {
        subscribe("user:$walletAddress")
    }

    /**
     * Register a message handler
     */
    fun onMessage(type: String, handler: (WsMessage) -> Unit): () -> Unit {
        messageHandlers.getOrPut(type) { mutableListOf() }.add(handler)
        
        // Return unsubscribe function
        return {
            messageHandlers[type]?.remove(handler)
        }
    }

    /**
     * Handle incoming message
     */
    private suspend fun handleMessage(text: String) {
        try {
            val message = json.decodeFromString<WsMessage>(text)
            
            // Call type-specific handlers
            messageHandlers[message::class.simpleName]?.forEach { handler ->
                handler(message)
            }
            
            // Call all handlers
            messageHandlers["*"]?.forEach { handler ->
                handler(message)
            }
        } catch (e: Exception) {
            // Handle parse error
        }
    }

    /**
     * Send a message to the server
     */
    private fun sendMessage(message: Map<String, String>) {
        webSocket?.let { ws ->
            val json = json.encodeToString(message)
            ws.send(json)
        }
    }

    /**
     * Cleanup resources
     */
    fun cleanup() {
        disconnect()
        scope.cancel()
    }
}

/**
 * Flow for event price updates
 */
fun MitraWebSocketClient.eventPriceFlow(eventId: String): Flow<Map<String, Double>> = flow {
    subscribeToEvent(eventId)
    
    val prices = MutableStateFlow<Map<String, Double>>(emptyMap())
    
    val unsubscribe = onMessage("PriceUpdate") { msg ->
        if (msg is WsMessage.PriceUpdate && msg.event_id == eventId) {
            prices.value = msg.prices
        }
    }
    
    try {
        prices.collect { emit(it) }
    } finally {
        unsubscribe()
        unsubscribe("event:$eventId")
    }
}

