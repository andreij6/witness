package app.witness.core.a2ui

import androidx.a2ui.model.processor.A2uiActionInterceptor
import androidx.a2ui.model.protocol.A2uiEventAction
import androidx.a2ui.model.protocol.A2uiUserAction

class WitnessActionInterceptor(private val sink: WitnessEventSink) : A2uiActionInterceptor {
    override suspend fun onInterceptAction(action: A2uiUserAction): A2uiUserAction? {
        if (action is A2uiEventAction) sink.dispatch(action.surfaceId, action.eventName, action.context)
        return null
    }
}
