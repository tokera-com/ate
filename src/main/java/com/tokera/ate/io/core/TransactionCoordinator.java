package com.tokera.ate.io.core;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.events.TokenStateChangedEvent;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.scopes.TokenScoped;

import javax.enterprise.event.Observes;
import java.util.HashSet;
import java.util.Set;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.ConcurrentSkipListSet;

/**
 * Coordinator that ensures pending transactions will be synchronized at user-defined points in the program
 */
@TokenScoped
public class TransactionCoordinator  {
    private AteDelegate d = AteDelegate.get();
    private ConcurrentLinkedQueue<QueuedSync> syncs = new ConcurrentLinkedQueue<>();

    class QueuedSync
    {
        final IPartitionKey partitionKey;
        final MessageSyncDto sync;

        public QueuedSync(IPartitionKey partitionKey, MessageSyncDto sync) {
            this.partitionKey = partitionKey;
            this.sync = sync;
        }
    }

    public void add(IPartitionKey partitionKey, MessageSyncDto sync)
    {
        syncs.add(new QueuedSync(partitionKey, sync));
    }

    public void finish()
    {
        while (true) {
            QueuedSync sync = syncs.poll();
            if (sync == null) return;
            d.io.sync(sync.partitionKey, sync.sync);
        }
    }

    public void onTokenChange(@Observes TokenStateChangedEvent event) {

        // Make sure any outstanding sync operations are fully executed for this topic
        if (d.currentToken.getWithinTokenScope()) {
            d.transaction.finish();
        }
    }
}
