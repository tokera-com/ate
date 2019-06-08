package com.tokera.examples.dao;

import com.tokera.ate.annotations.PermitParentType;
import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.units.DaoId;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.Dependent;
import java.math.BigDecimal;
import java.util.*;

@Dependent
@PermitParentType(Account.class)
public class MonthlyActivity extends BaseDao {
    public UUID id;
    public UUID account;
    public Date start;
    public Date end;
    public final List<Transaction> transactions = new LinkedList<Transaction>();
    public final Map<String, BigDecimal> balances = new TreeMap<String, BigDecimal>();

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MonthlyActivity() {
    }

    public MonthlyActivity(Account acc, Date start, Date end) {
        this.id = UUID.randomUUID();
        this.account = acc.id;
        this.start = start;
        this.end = end;
    }

    public @DaoId UUID getId() {
        return this.id;
    }

    public @Nullable @DaoId UUID getParentId() {
        return this.account;
    }

    public List<Transaction> getTransactions() { return this.transactions; }

    public Map<String, BigDecimal> getBalances() { return this.balances; }
}
